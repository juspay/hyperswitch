use std::{num::Wrapping, str};

use error_stack::{report, IntoReport, ResultExt};
#[cfg(feature = "basilisk")]
use josekit::jwe;
use rand;
use ring::{aead::*, error::Unspecified};

use crate::{
    configs::settings::Jwekey,
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

pub struct KeyHandler;

#[cfg(feature = "kms")]
mod kms {
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_kms::{types::Blob, Client, Region};
    use base64::Engine;

    use super::*;
    use crate::consts;

    impl KeyHandler {
        // Fetching KMS decrypted key
        // | Amazon KMS decryption
        // This expect a base64 encoded input but we values are set via aws cli in env than cli
        // already does that so we don't need to
        pub async fn get_kms_decrypted_key(
            aws_keys: &Jwekey,
            kms_enc_key: String,
        ) -> CustomResult<String, errors::EncryptionError> {
            let region = aws_keys.aws_region.to_string();
            let key_id = aws_keys.aws_key_id.clone();
            let region_provider = RegionProviderChain::first_try(Region::new(region));
            let shared_config = aws_config::from_env().region(region_provider).load().await;
            let client = Client::new(&shared_config);
            let data = consts::BASE64_ENGINE
                .decode(kms_enc_key)
                .into_report()
                .change_context(errors::EncryptionError)
                .attach_printable("Error decoding from base64")?;
            let blob = Blob::new(data);
            let resp = client
                .decrypt()
                .key_id(key_id)
                .ciphertext_blob(blob)
                .send()
                .await
                .into_report()
                .change_context(errors::EncryptionError)
                .attach_printable("Error decrypting kms encrypted data")?;
            match resp.plaintext() {
                Some(inner) => {
                    let bytes = inner.as_ref().to_vec();
                    let res = String::from_utf8(bytes)
                        .into_report()
                        .change_context(errors::EncryptionError)
                        .attach_printable("Could not convert to UTF-8")?;
                    Ok(res)
                }
                None => Err(report!(errors::EncryptionError)
                    .attach_printable("Missing plaintext in response")),
            }
        }
    }
}

#[cfg(not(feature = "kms"))]
impl KeyHandler {
    pub async fn get_kms_decrypted_key(
        _aws_keys: &Jwekey,
        key: String,
    ) -> CustomResult<String, errors::EncryptionError> {
        Ok(key)
    }
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

pub fn get_key_id(keys: &Jwekey) -> &str {
    let key_identifier = "1"; // [#46]: Fetch this value from redis or external sources
    if key_identifier == "1" {
        &keys.locker_key_identifier1
    } else {
        &keys.locker_key_identifier2
    }
}

#[cfg(feature = "basilisk")]
pub async fn encrypt_jwe(
    keys: &Jwekey,
    msg: &str,
) -> CustomResult<String, errors::EncryptionError> {
    let alg = jwe::RSA_OAEP_256;
    let key_id = get_key_id(keys);
    let public_key = if key_id == keys.locker_key_identifier1 {
        KeyHandler::get_kms_decrypted_key(keys, keys.locker_encryption_key1.to_string()).await?
    } else {
        KeyHandler::get_kms_decrypted_key(keys, keys.locker_encryption_key2.to_string()).await?
    };
    let payload = msg.as_bytes();
    let enc = "A256GCM";
    let mut src_header = jwe::JweHeader::new();
    src_header.set_content_encryption(enc);
    src_header.set_token_type("JWT");
    let encrypter = alg
        .encrypter_from_pem(public_key)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting JweEncryptor")?;
    let jwt = jwe::serialize_compact(payload, &src_header, &encrypter)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting jwt string")?;
    Ok(jwt)
}

pub async fn decrypt_jwe(
    keys: &Jwekey,
    jwt: &str,
    resp_key_id: &str,
) -> CustomResult<String, errors::EncryptionError> {
    let alg = jwe::RSA_OAEP_256;
    let key_id = get_key_id(keys);
    let private_key = if key_id == keys.locker_key_identifier1 {
        KeyHandler::get_kms_decrypted_key(keys, keys.locker_decryption_key1.to_string()).await?
    } else {
        KeyHandler::get_kms_decrypted_key(keys, keys.locker_decryption_key2.to_string()).await?
    };

    let decrypter = alg
        .decrypter_from_pem(private_key)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting JweDecryptor")?;

    let (dst_payload, _dst_header) = jwe::deserialize_compact(jwt, &decrypter)
        .into_report()
        .change_context(errors::EncryptionError)
        .attach_printable("Error getting Decrypted jwe")?;
    utils::when(resp_key_id.ne(key_id), || {
        Err(report!(errors::EncryptionError).attach_printable("Missing ciphertext blob"))
            .attach_printable("key_id mismatch, Error authenticating response")
    })?;
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
    async fn test_jwe() {
        let conf = settings::Settings::new().unwrap();
        let jwt = encrypt_jwe(&conf.jwekey, "request_payload").await.unwrap();
        let payload = decrypt_jwe(&conf.jwekey, &jwt, &conf.jwekey.locker_key_identifier1)
            .await
            .unwrap();
        assert_eq!("request_payload".to_string(), payload)
    }
}
