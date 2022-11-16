use std::{num::Wrapping, str};

use error_stack::{IntoReport, ResultExt};
use rand;
use ring::{aead::*, error::Unspecified};

use crate::{
    configs::settings::Keys,
    core::errors::{self, CustomResult},
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

pub struct KeyHandler {}

#[cfg(feature = "kms")]
use aws_config::meta::region::RegionProviderChain;
#[cfg(feature = "kms")]
use aws_sdk_kms::{types::Blob, Client, Region};
#[cfg(feature = "kms")]
use error_stack::report;
#[cfg(feature = "kms")]
impl KeyHandler {
    // Fetching KMS decrypted key
    // | Amazon KMS decryption
    //This expect a base64 encoded input but we values are set via aws cli in env than cli already does that so we don't need to
    pub async fn get_encryption_key(keys: &Keys) -> CustomResult<String, errors::EncryptionError> {
        let kms_enc_key = keys.temp_card_key.to_string();
        let region = keys.aws_region.to_string();
        let key_id = keys.aws_key_id.clone();
        let region_provider = RegionProviderChain::first_try(Region::new(region));
        let shared_config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&shared_config);
        let data = base64::decode(kms_enc_key)
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
            None => {
                Err(report!(errors::EncryptionError)
                    .attach_printable("Missing plaintext in response"))
            }
        }
    }
    pub async fn set_encryption_key(
        input: &str,
        keys: &Keys,
    ) -> CustomResult<String, errors::EncryptionError> {
        let region = keys.aws_region.to_string();
        let key_id = keys.aws_key_id.clone();
        let region_provider = RegionProviderChain::first_try(Region::new(region));
        let shared_config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&shared_config);
        let blob = Blob::new(input.as_bytes());
        let resp = client
            .encrypt()
            .key_id(key_id)
            .plaintext(blob)
            .send()
            .await
            .into_report()
            .change_context(errors::EncryptionError)
            .attach_printable("Error getting EncryptOutput")?;
        match resp.ciphertext_blob {
            Some(blob) => {
                let bytes = blob.as_ref();
                let encoded_res = base64::encode(bytes);
                Ok(encoded_res)
            }
            None => {
                Err(report!(errors::EncryptionError).attach_printable("Missing ciphertext blob"))
            }
        }
    }
}
#[cfg(not(feature = "kms"))]
impl KeyHandler {
    // Fetching KMS decrypted key
    pub async fn get_encryption_key(keys: &Keys) -> CustomResult<String, errors::EncryptionError> {
        Ok(keys.temp_card_key.clone())
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::utils::{self, ValueExt};
    #[cfg(feature = "kms")]
    use crate::Settings;

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

    #[cfg(feature = "kms")]
    #[actix_rt::test]
    #[ignore]
    async fn test_kms() {
        let conf = Settings::new().unwrap();
        let kms_encrypted = KeyHandler::get_encryption_key(&conf.keys)
            .await
            .expect("Error encode_kms");
        let kms_decrypted = KeyHandler::set_encryption_key(&kms_encrypted, &conf.keys)
            .await
            .expect("error decode_kms");
        assert_eq!("Testing KMS".to_string(), kms_decrypted)
    }
}
