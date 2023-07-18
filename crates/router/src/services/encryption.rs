use std::{num::Wrapping, str};

use error_stack::{report, IntoReport, ResultExt};
use josekit::{jwe, jws};
use rand;
use ring::{aead::*, error::Unspecified};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors::{self, CustomResult},
    utils,
};

struct NonceGen {
    current: Wrapping<u128>,
    start: u128,
}

impl NonceGen {
    fn new(start: [u8; 12]) -> Self {
        let mut array = [0; 16];
        array[..12].copy_from_slice(&start);
        let start = if cfg!(target_endian = "little") {
            u128::from_le_bytes(array)
        } else {
            u128::from_be_bytes(array)
        };
        Self {
            current: Wrapping(start),
            start,
        }
    }
}

impl NonceSequence for NonceGen {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        let n = self.current.0;
        self.current += 1;
        if self.current.0 == self.start {
            return Err(Unspecified);
        }
        let value = if cfg!(target_endian = "little") {
            n.to_le_bytes()[..12].try_into()?
        } else {
            n.to_be_bytes()[..12].try_into()?
        };
        let nonce = Nonce::assume_unique_for_key(value);
        Ok(nonce)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwsBody {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JweBody {
    pub header: String,
    pub iv: String,
    pub encrypted_payload: String,
    pub tag: String,
    pub encrypted_key: String,
}

pub fn encrypt(msg: &String, key: &[u8]) -> CustomResult<Vec<u8>, errors::EncryptionError> {
    let nonce_seed = rand::random();
    let mut sealing_key = {
        let key = UnboundKey::new(&AES_256_GCM, key)
            .map_err(errors::EncryptionError::from)
            .into_report()
            .attach_printable("Unbound Key Error")?;
        let nonce_gen = NonceGen::new(nonce_seed);
        SealingKey::new(key, nonce_gen)
    };
    let msg_byte = msg.as_bytes();
    let mut data = msg_byte.to_vec();

    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut data)
        .map_err(errors::EncryptionError::from)
        .into_report()
        .attach_printable("Error Encrypting")?;
    let nonce_vec = nonce_seed.to_vec();
    data.splice(0..0, nonce_vec); //prepend nonce at the start
    Ok(data)
}

pub fn decrypt(mut data: Vec<u8>, key: &[u8]) -> CustomResult<String, errors::EncryptionError> {
    let nonce_seed = data[0..12]
        .try_into()
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting nonce")?;
    data.drain(0..12);

    let mut opening_key = {
        let key = UnboundKey::new(&AES_256_GCM, key)
            .map_err(errors::EncryptionError::from)
            .into_report()
            .attach_printable("Unbound Key Error")?;
        let nonce_gen = NonceGen::new(nonce_seed);
        OpeningKey::new(key, nonce_gen)
    };
    let res_byte = opening_key
        .open_in_place(Aad::empty(), &mut data)
        .map_err(errors::EncryptionError::from)
        .into_report()
        .attach_printable("Error Decrypting")?;
    let response = str::from_utf8_mut(res_byte)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error from_utf8")?;
    Ok(response.to_string())
}

pub async fn encrypt_jwe(
    payload: &[u8],
    public_key: impl AsRef<[u8]>,
) -> CustomResult<String, errors::EncryptionError> {
    let alg = jwe::RSA_OAEP_256;
    let enc = "A256GCM";
    let mut src_header = jwe::JweHeader::new();
    src_header.set_content_encryption(enc);
    src_header.set_token_type("JWT");
    let encrypter = alg
        .encrypter_from_pem(public_key)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting JweEncryptor")?;

    jwe::serialize_compact(payload, &src_header, &encrypter)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting jwt string")
}

pub enum KeyIdCheck<'a> {
    RequestResponseKeyId((&'a str, &'a str)),
    SkipKeyIdCheck,
}

pub async fn decrypt_jwe(
    jwt: &str,
    key_ids: KeyIdCheck<'_>,
    private_key: impl AsRef<[u8]>,
    alg: jwe::alg::rsaes::RsaesJweAlgorithm,
) -> CustomResult<String, errors::EncryptionError> {
    if let KeyIdCheck::RequestResponseKeyId((req_key_id, resp_key_id)) = key_ids {
        utils::when(req_key_id.ne(resp_key_id), || {
            Err(report!(errors::EncryptionError)
                .attach_printable("key_id mismatch, Error authenticating response"))
        })?;
    }

    let decrypter = alg
        .decrypter_from_pem(private_key)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting JweDecryptor")?;

    let (dst_payload, _dst_header) = jwe::deserialize_compact(jwt, &decrypter)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting Decrypted jwe")?;

    String::from_utf8(dst_payload)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Could not decode JWE payload from UTF-8")
}

pub async fn jws_sign_payload(
    payload: &[u8],
    kid: &str,
    private_key: impl AsRef<[u8]>,
) -> CustomResult<String, errors::EncryptionError> {
    let alg = jws::RS256;
    let mut src_header = jws::JwsHeader::new();
    src_header.set_key_id(kid);
    let signer = alg
        .signer_from_pem(private_key)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting signer")?;
    let jwt = jws::serialize_compact(payload, &src_header, &signer)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting signed jwt string")?;
    Ok(jwt)
}

pub fn verify_sign(
    jws_body: String,
    key: impl AsRef<[u8]>,
) -> CustomResult<String, errors::EncryptionError> {
    let alg = jws::RS256;
    let input = jws_body.as_bytes();
    let verifier = alg
        .verifier_from_pem(key)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting verifier")?;
    let (dst_payload, _dst_header) = jws::deserialize_compact(input, &verifier)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting Decrypted jws")?;
    let resp = String::from_utf8(dst_payload)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Could not convert to UTF-8")?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::{
        configs::settings,
        utils::{self, ValueExt},
    };

    fn generate_key() -> [u8; 32] {
        let key: [u8; 32] = rand::random();
        key
    }

    #[test]
    fn test_enc() {
        let key = generate_key();
        let enc_data = encrypt(&"Test_Encrypt".to_string(), &key).unwrap();
        let card_info = utils::Encode::<Vec<u8>>::encode_to_value(&enc_data).unwrap();
        let data: Vec<u8> = card_info.parse_value("ParseEncryptedData").unwrap();
        let dec_data = decrypt(data, &key).unwrap();
        assert_eq!(dec_data, "Test_Encrypt".to_string());
    }

    #[actix_rt::test]
    #[ignore]
    async fn test_jwe() {
        let conf = settings::Settings::new().unwrap();
        let jwt = encrypt_jwe(
            "request_payload".as_bytes(),
            conf.jwekey.locker_encryption_key1.to_owned(),
        )
        .await
        .unwrap();
        let alg = jwe::RSA_OAEP_256;
        let payload = decrypt_jwe(
            &jwt,
            KeyIdCheck::SkipKeyIdCheck,
            conf.jwekey.locker_decryption_key1.to_owned(),
            alg,
        )
        .await
        .unwrap();
        assert_eq!("request_payload".to_string(), payload)
    }

    #[actix_rt::test]
    #[ignore]
    async fn test_jws() {
        let conf = settings::Settings::new().unwrap();
        let jwt = jws_sign_payload("jws payload".as_bytes(), "1", conf.jwekey.vault_private_key)
            .await
            .unwrap();
        let payload = verify_sign(jwt, &conf.jwekey.vault_encryption_key).unwrap();
        assert_eq!("jws payload".to_string(), payload)
    }
}
