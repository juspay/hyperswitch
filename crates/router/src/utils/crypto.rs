use error_stack::{IntoReport, ResultExt};
use ring::{aead, hmac};

use crate::core::errors::{self, CustomResult};

const RING_ERR_UNSPECIFIED: &str = "ring::error::Unspecified";

pub trait SignMessage {
    fn sign_message(
        &self,
        _secret: &[u8],
        _msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError>;
}

pub trait VerifySignature {
    fn verify_signature(
        &self,
        _secret: &[u8],
        _signature: &[u8],
        _msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError>;
}

pub trait EncodeMessage {
    fn encode_message(
        &self,
        _secret: &[u8],
        _msg: &[u8],
    ) -> CustomResult<(Vec<u8>, Vec<u8>), errors::CryptoError>;
}

pub trait DecodeMessage {
    fn decode_message(
        &self,
        _secret: &[u8],
        _msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError>;
}

pub struct NoAlgorithm;

impl SignMessage for NoAlgorithm {
    fn sign_message(
        &self,
        _secret: &[u8],
        _msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        Ok(Vec::new())
    }
}

impl VerifySignature for NoAlgorithm {
    fn verify_signature(
        &self,
        _secret: &[u8],
        _signature: &[u8],
        _msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        Ok(true)
    }
}

impl EncodeMessage for NoAlgorithm {
    fn encode_message(
        &self,
        _secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<(Vec<u8>, Vec<u8>), errors::CryptoError> {
        Ok((msg.to_vec(), Vec::new()))
    }
}

impl DecodeMessage for NoAlgorithm {
    fn decode_message(
        &self,
        _secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        Ok(msg.to_vec())
    }
}

pub struct HmacSha256;

impl SignMessage for HmacSha256 {
    fn sign_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        Ok(hmac::sign(&key, msg).as_ref().to_vec())
    }
}

impl VerifySignature for HmacSha256 {
    fn verify_signature(
        &self,
        secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);

        Ok(hmac::verify(&key, msg, signature).is_ok())
    }
}

pub struct HmacSha512;

impl SignMessage for HmacSha512 {
    fn sign_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let key = hmac::Key::new(hmac::HMAC_SHA512, secret);
        Ok(hmac::sign(&key, msg).as_ref().to_vec())
    }
}

impl VerifySignature for HmacSha512 {
    fn verify_signature(
        &self,
        secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let key = hmac::Key::new(hmac::HMAC_SHA512, secret);

        Ok(hmac::verify(&key, msg, signature).is_ok())
    }
}

pub struct GcmAes256 {
    nonce: Vec<u8>,
}

impl EncodeMessage for GcmAes256 {
    fn encode_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<(Vec<u8>, Vec<u8>), errors::CryptoError> {
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, secret)
            .map_err(|_| errors::CryptoError::EncodingFailed)
            .into_report()
            .attach_printable(RING_ERR_UNSPECIFIED)?;

        let nonce = aead::Nonce::try_assume_unique_for_key(&self.nonce)
            .map_err(|_| errors::CryptoError::EncodingFailed)
            .into_report()
            .attach_printable(RING_ERR_UNSPECIFIED)?;

        let sealing_key = aead::LessSafeKey::new(unbound_key);
        let mut mutable_msg = msg.to_vec();

        let tag = sealing_key
            .seal_in_place_separate_tag(nonce, aead::Aad::empty(), &mut mutable_msg)
            .map_err(|_| errors::CryptoError::EncodingFailed)
            .into_report()
            .attach_printable(RING_ERR_UNSPECIFIED)?;

        Ok((mutable_msg, tag.as_ref().to_vec()))
    }
}

impl DecodeMessage for GcmAes256 {
    fn decode_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, secret)
            .map_err(|_| errors::CryptoError::DecodingFailed)
            .into_report()
            .attach_printable(RING_ERR_UNSPECIFIED)?;

        let nonce = aead::Nonce::try_assume_unique_for_key(&self.nonce)
            .map_err(|_| errors::CryptoError::DecodingFailed)
            .into_report()
            .attach_printable(RING_ERR_UNSPECIFIED)?;

        let opening_key = aead::LessSafeKey::new(unbound_key);

        let mut mutable_msg = msg.to_vec();

        let output = opening_key
            .open_in_place(nonce, aead::Aad::empty(), &mut mutable_msg)
            .map_err(|_| errors::CryptoError::DecodingFailed)
            .into_report()
            .attach_printable(RING_ERR_UNSPECIFIED)?;

        Ok(output.to_vec())
    }
}

#[cfg(test)]
mod crypto_tests {
    #![allow(clippy::expect_used)]
    use super::{DecodeMessage, EncodeMessage, SignMessage, VerifySignature};

    #[test]
    fn test_hmac_sha256_sign_message() {
        let message = r#"{"type":"payment_intent"}"#.as_bytes();
        let secret = "hmac_secret_1234".as_bytes();
        let right_signature =
            hex::decode("d5550730377011948f12cc28889bee590d2a5434d6f54b87562f2dbc2657823e")
                .expect("Right signature decoding");

        let signature = super::HmacSha256
            .sign_message(secret, message)
            .expect("Signature");

        assert_eq!(signature, right_signature);
    }

    #[test]
    fn test_hmac_sha256_verify_signature() {
        let right_signature =
            hex::decode("d5550730377011948f12cc28889bee590d2a5434d6f54b87562f2dbc2657823e")
                .expect("Right signature decoding");
        let wrong_signature =
            hex::decode("d5550730377011948f12cc28889bee590d2a5434d6f54b87562f2dbc2657823f")
                .expect("Wrong signature decoding");
        let secret = "hmac_secret_1234".as_bytes();
        let data = r#"{"type":"payment_intent"}"#.as_bytes();

        let right_verified = super::HmacSha256
            .verify_signature(secret, &right_signature, data)
            .expect("Right signature verification result");

        assert!(right_verified);

        let wrong_verified = super::HmacSha256
            .verify_signature(secret, &wrong_signature, data)
            .expect("Wrong signature verification result");

        assert!(!wrong_verified);
    }

    #[test]
    fn test_hmac_sha512_sign_message() {
        let message = r#"{"type":"payment_intent"}"#.as_bytes();
        let secret = "hmac_secret_1234".as_bytes();
        let right_signature = hex::decode("38b0bc1ea66b14793e39cd58e93d37b799a507442d0dd8d37443fa95dec58e57da6db4742636fea31201c48e57a66e73a308a2e5a5c6bb831e4e39fe2227c00f")
            .expect("signature decoding");

        let signature = super::HmacSha512
            .sign_message(secret, message)
            .expect("Signature");

        assert_eq!(signature, right_signature);
    }

    #[test]
    fn test_hmac_sha512_verify_signature() {
        let right_signature = hex::decode("38b0bc1ea66b14793e39cd58e93d37b799a507442d0dd8d37443fa95dec58e57da6db4742636fea31201c48e57a66e73a308a2e5a5c6bb831e4e39fe2227c00f")
            .expect("signature decoding");
        let wrong_signature =
            hex::decode("d5550730377011948f12cc28889bee590d2a5434d6f54b87562f2dbc2657823f")
                .expect("Wrong signature decoding");
        let secret = "hmac_secret_1234".as_bytes();
        let data = r#"{"type":"payment_intent"}"#.as_bytes();

        let right_verified = super::HmacSha512
            .verify_signature(secret, &right_signature, data)
            .expect("Right signature verification result");

        assert!(right_verified);

        let wrong_verified = super::HmacSha256
            .verify_signature(secret, &wrong_signature, data)
            .expect("Wrong signature verification result");

        assert!(!wrong_verified);
    }

    #[test]
    fn test_gcm_aes_256_encode_message() {
        let message = r#"{"type":"PAYMENT"}"#.as_bytes();
        let secret =
            hex::decode("000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f")
                .expect("Secret decoding");
        let nonce = hex::decode("000000000000000000000000").expect("Nonce hex decoding");
        let actual_encoded_message =
            hex::decode("0A3471C72D9BE49A8520F79C66BBD9A12FF9").expect("Message decoding");
        let actual_auth_tag =
            hex::decode("CE573FB7A41AB78E743180DC83FF09BD").expect("Auth tag decoding");

        let algorithm = super::GcmAes256 {
            nonce: nonce.to_vec(),
        };

        let (encoded_message, auth_tag) = algorithm
            .encode_message(&secret, message)
            .expect("Encoded message and tag");

        assert_eq!(encoded_message, actual_encoded_message);
        assert_eq!(auth_tag, actual_auth_tag);
    }

    #[test]
    fn test_gcm_aes_256_decode_message() {
        let right_secret =
            hex::decode("000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f")
                .expect("Secret decoding");
        let wrong_secret =
            hex::decode("000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0e")
                .expect("Secret decoding");
        let nonce = hex::decode("000000000000000000000000").expect("Nonce hex decoding");
        let mut auth_tag =
            hex::decode("CE573FB7A41AB78E743180DC83FF09BD").expect("Auth tag decoding");
        let mut message =
            hex::decode("0A3471C72D9BE49A8520F79C66BBD9A12FF9").expect("Message decoding");

        message.append(&mut auth_tag);

        let algorithm = super::GcmAes256 {
            nonce: nonce.to_vec(),
        };

        let decoded = algorithm
            .decode_message(&right_secret, &message)
            .expect("Decoded message");

        assert_eq!(decoded, r#"{"type":"PAYMENT"}"#.as_bytes());

        let err_decoded = algorithm.decode_message(&wrong_secret, &message);

        assert!(err_decoded.is_err());
    }
}
