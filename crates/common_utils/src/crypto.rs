//! Utilities for cryptographic algorithms
use std::ops::Deref;

use base64::Engine;
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};
use ring::{
    aead::{self, BoundKey, OpeningKey, SealingKey, UnboundKey},
    hmac, rand as ring_rand,
    signature::{RsaKeyPair, RSA_PSS_SHA256},
};
#[cfg(feature = "logs")]
use router_env::logger;
use rsa::{pkcs8::DecodePublicKey, signature::Verifier};

use crate::{
    consts::BASE64_ENGINE,
    errors::{self, CustomResult},
    pii::{self, EncryptionStrategy},
};

#[derive(Clone, Debug)]
struct NonceSequence(u128);

impl NonceSequence {
    /// Byte index at which sequence number starts in a 16-byte (128-bit) sequence.
    /// This byte index considers the big endian order used while encoding and decoding the nonce
    /// to/from a 128-bit unsigned integer.
    const SEQUENCE_NUMBER_START_INDEX: usize = 4;

    /// Generate a random nonce sequence.
    fn new() -> Result<Self, ring::error::Unspecified> {
        use ring::rand::{SecureRandom, SystemRandom};

        let rng = SystemRandom::new();

        // 96-bit sequence number, stored in a 128-bit unsigned integer in big-endian order
        let mut sequence_number = [0_u8; 128 / 8];
        rng.fill(&mut sequence_number[Self::SEQUENCE_NUMBER_START_INDEX..])?;
        let sequence_number = u128::from_be_bytes(sequence_number);

        Ok(Self(sequence_number))
    }

    /// Returns the current nonce value as bytes.
    fn current(&self) -> [u8; aead::NONCE_LEN] {
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&self.0.to_be_bytes()[Self::SEQUENCE_NUMBER_START_INDEX..]);
        nonce
    }

    /// Constructs a nonce sequence from bytes
    fn from_bytes(bytes: [u8; aead::NONCE_LEN]) -> Self {
        let mut sequence_number = [0_u8; 128 / 8];
        sequence_number[Self::SEQUENCE_NUMBER_START_INDEX..].copy_from_slice(&bytes);
        let sequence_number = u128::from_be_bytes(sequence_number);
        Self(sequence_number)
    }
}

impl aead::NonceSequence for NonceSequence {
    fn advance(&mut self) -> Result<aead::Nonce, ring::error::Unspecified> {
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&self.0.to_be_bytes()[Self::SEQUENCE_NUMBER_START_INDEX..]);

        // Increment sequence number
        self.0 = self.0.wrapping_add(1);

        // Return previous sequence number as bytes
        Ok(aead::Nonce::assume_unique_for_key(nonce))
    }
}

/// Trait for cryptographically signing messages
pub trait SignMessage {
    /// Takes in a secret and a message and returns the calculated signature as bytes
    fn sign_message(
        &self,
        _secret: &[u8],
        _msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError>;
}

/// Trait for cryptographically verifying a message against a signature
pub trait VerifySignature {
    /// Takes in a secret, the signature and the message and verifies the message
    /// against the signature
    fn verify_signature(
        &self,
        _secret: &[u8],
        _signature: &[u8],
        _msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError>;
}

/// Trait for cryptographically encoding a message
pub trait EncodeMessage {
    /// Takes in a secret and the message and encodes it, returning bytes
    fn encode_message(
        &self,
        _secret: &[u8],
        _msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError>;
}

/// Trait for cryptographically decoding a message
pub trait DecodeMessage {
    /// Takes in a secret, an encoded messages and attempts to decode it, returning bytes
    fn decode_message(
        &self,
        _secret: &[u8],
        _msg: Secret<Vec<u8>, EncryptionStrategy>,
    ) -> CustomResult<Vec<u8>, errors::CryptoError>;
}

/// Represents no cryptographic algorithm.
/// Implements all crypto traits and acts like a Nop
#[derive(Debug)]
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
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        Ok(msg.to_vec())
    }
}

impl DecodeMessage for NoAlgorithm {
    fn decode_message(
        &self,
        _secret: &[u8],
        msg: Secret<Vec<u8>, EncryptionStrategy>,
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        Ok(msg.expose())
    }
}

/// Represents the HMAC-SHA-1 algorithm
#[derive(Debug)]
pub struct HmacSha1;

impl SignMessage for HmacSha1 {
    fn sign_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
        Ok(hmac::sign(&key, msg).as_ref().to_vec())
    }
}

impl VerifySignature for HmacSha1 {
    fn verify_signature(
        &self,
        secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);

        Ok(hmac::verify(&key, msg, signature).is_ok())
    }
}

/// Represents the HMAC-SHA-256 algorithm
#[derive(Debug)]
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

/// Represents the HMAC-SHA-512 algorithm
#[derive(Debug)]
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

/// Blake3
#[derive(Debug)]
pub struct Blake3(String);

impl Blake3 {
    /// Create a new instance of Blake3 with a key
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

impl SignMessage for Blake3 {
    fn sign_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let key = blake3::derive_key(&self.0, secret);
        let output = blake3::keyed_hash(&key, msg).as_bytes().to_vec();
        Ok(output)
    }
}

impl VerifySignature for Blake3 {
    fn verify_signature(
        &self,
        secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let key = blake3::derive_key(&self.0, secret);
        let output = blake3::keyed_hash(&key, msg);
        Ok(output.as_bytes() == signature)
    }
}

/// Represents the GCM-AES-256 algorithm
#[derive(Debug)]
pub struct GcmAes256;

impl EncodeMessage for GcmAes256 {
    fn encode_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let nonce_sequence =
            NonceSequence::new().change_context(errors::CryptoError::EncodingFailed)?;
        let current_nonce = nonce_sequence.current();
        let key = UnboundKey::new(&aead::AES_256_GCM, secret)
            .change_context(errors::CryptoError::EncodingFailed)?;
        let mut key = SealingKey::new(key, nonce_sequence);
        let mut in_out = msg.to_vec();

        key.seal_in_place_append_tag(aead::Aad::empty(), &mut in_out)
            .change_context(errors::CryptoError::EncodingFailed)?;
        in_out.splice(0..0, current_nonce);

        Ok(in_out)
    }
}

impl DecodeMessage for GcmAes256 {
    fn decode_message(
        &self,
        secret: &[u8],
        msg: Secret<Vec<u8>, EncryptionStrategy>,
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let msg = msg.expose();
        let key = UnboundKey::new(&aead::AES_256_GCM, secret)
            .change_context(errors::CryptoError::DecodingFailed)?;

        let nonce_sequence = NonceSequence::from_bytes(
            <[u8; aead::NONCE_LEN]>::try_from(
                msg.get(..aead::NONCE_LEN)
                    .ok_or(errors::CryptoError::DecodingFailed)
                    .attach_printable("Failed to read the nonce form the encrypted ciphertext")?,
            )
            .change_context(errors::CryptoError::DecodingFailed)?,
        );

        let mut key = OpeningKey::new(key, nonce_sequence);
        let mut binding = msg;
        let output = binding.as_mut_slice();

        let result = key
            .open_within(aead::Aad::empty(), output, aead::NONCE_LEN..)
            .change_context(errors::CryptoError::DecodingFailed)?;

        Ok(result.to_vec())
    }
}
/// Represents the ED25519 signature verification algorithm
#[derive(Debug)]
pub struct Ed25519;

impl Ed25519 {
    /// ED25519 algorithm constants
    const ED25519_PUBLIC_KEY_LEN: usize = 32;
    const ED25519_SIGNATURE_LEN: usize = 64;
    /// Validates ED25519 inputs (public key and signature lengths)
    fn validate_inputs(
        public_key: &[u8],
        signature: &[u8],
    ) -> CustomResult<(), errors::CryptoError> {
        // Validate public key length
        if public_key.len() != Self::ED25519_PUBLIC_KEY_LEN {
            return Err(errors::CryptoError::InvalidKeyLength).attach_printable(format!(
                "Invalid ED25519 public key length: expected {} bytes, got {}",
                Self::ED25519_PUBLIC_KEY_LEN,
                public_key.len()
            ));
        }

        // Validate signature length
        if signature.len() != Self::ED25519_SIGNATURE_LEN {
            return Err(errors::CryptoError::InvalidKeyLength).attach_printable(format!(
                "Invalid ED25519 signature length: expected {} bytes, got {}",
                Self::ED25519_SIGNATURE_LEN,
                signature.len()
            ));
        }

        Ok(())
    }
}

impl VerifySignature for Ed25519 {
    fn verify_signature(
        &self,
        public_key: &[u8],
        signature: &[u8], // ED25519 signature bytes (must be 64 bytes)
        msg: &[u8],       // Message that was signed
    ) -> CustomResult<bool, errors::CryptoError> {
        // Validate inputs first
        Self::validate_inputs(public_key, signature)?;

        // Create unparsed public key
        let ring_public_key =
            ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key);

        // Perform verification
        match ring_public_key.verify(msg, signature) {
            Ok(()) => Ok(true),
            Err(_err) => {
                #[cfg(feature = "logs")]
                logger::error!("ED25519 signature verification failed: {:?}", _err);
                Err(errors::CryptoError::SignatureVerificationFailed)
                    .attach_printable("ED25519 signature verification failed")
            }
        }
    }
}

impl SignMessage for Ed25519 {
    fn sign_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        if secret.len() != 32 {
            return Err(errors::CryptoError::InvalidKeyLength).attach_printable(format!(
                "Invalid ED25519 private key length: expected 32 bytes, got {}",
                secret.len()
            ));
        }
        let key_pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(secret)
            .change_context(errors::CryptoError::MessageSigningFailed)
            .attach_printable("Failed to create ED25519 key pair from seed")?;

        let signature = key_pair.sign(msg);
        Ok(signature.as_ref().to_vec())
    }
}
/// Secure Hash Algorithm 512
#[derive(Debug)]
pub struct Sha512;

/// Secure Hash Algorithm 256
#[derive(Debug)]
pub struct Sha256;

/// Trait for generating a digest for SHA
pub trait GenerateDigest {
    /// takes a message and creates a digest for it
    fn generate_digest(&self, message: &[u8]) -> CustomResult<Vec<u8>, errors::CryptoError>;
}

impl GenerateDigest for Sha512 {
    fn generate_digest(&self, message: &[u8]) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let digest = ring::digest::digest(&ring::digest::SHA512, message);
        Ok(digest.as_ref().to_vec())
    }
}
impl VerifySignature for Sha512 {
    fn verify_signature(
        &self,
        _secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let msg_str = std::str::from_utf8(msg)
            .change_context(errors::CryptoError::EncodingFailed)?
            .to_owned();
        let hashed_digest = hex::encode(
            Self.generate_digest(msg_str.as_bytes())
                .change_context(errors::CryptoError::SignatureVerificationFailed)?,
        );
        let hashed_digest_into_bytes = hashed_digest.into_bytes();
        Ok(hashed_digest_into_bytes == signature)
    }
}
/// MD5 hash function
#[derive(Debug)]
pub struct Md5;

impl GenerateDigest for Md5 {
    fn generate_digest(&self, message: &[u8]) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let digest = md5::compute(message);
        Ok(digest.as_ref().to_vec())
    }
}

impl VerifySignature for Md5 {
    fn verify_signature(
        &self,
        _secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let hashed_digest = Self
            .generate_digest(msg)
            .change_context(errors::CryptoError::SignatureVerificationFailed)?;
        Ok(hashed_digest == signature)
    }
}

impl GenerateDigest for Sha256 {
    fn generate_digest(&self, message: &[u8]) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let digest = ring::digest::digest(&ring::digest::SHA256, message);
        Ok(digest.as_ref().to_vec())
    }
}

impl VerifySignature for Sha256 {
    fn verify_signature(
        &self,
        _secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        let hashed_digest = Self
            .generate_digest(msg)
            .change_context(errors::CryptoError::SignatureVerificationFailed)?;
        let hashed_digest_into_bytes = hashed_digest.as_slice();
        Ok(hashed_digest_into_bytes == signature)
    }
}

/// Secure Hash Algorithm 256 with RSA public-key cryptosystem
#[derive(Debug)]
pub struct RsaSha256;

impl VerifySignature for RsaSha256 {
    fn verify_signature(
        &self,
        secret: &[u8],
        signature: &[u8],
        msg: &[u8],
    ) -> CustomResult<bool, errors::CryptoError> {
        // create verifying key
        let decoded_public_key = BASE64_ENGINE
            .decode(secret)
            .change_context(errors::CryptoError::SignatureVerificationFailed)
            .attach_printable("base64 decoding failed")?;

        let string_public_key = String::from_utf8(decoded_public_key)
            .change_context(errors::CryptoError::SignatureVerificationFailed)
            .attach_printable("utf8 to string parsing failed")?;

        let rsa_public_key = rsa::RsaPublicKey::from_public_key_pem(&string_public_key)
            .change_context(errors::CryptoError::SignatureVerificationFailed)
            .attach_printable("rsa public key transformation failed")?;

        let verifying_key = rsa::pkcs1v15::VerifyingKey::<rsa::sha2::Sha256>::new(rsa_public_key);

        // transfrom the signature
        let decoded_signature = BASE64_ENGINE
            .decode(signature)
            .change_context(errors::CryptoError::SignatureVerificationFailed)
            .attach_printable("base64 decoding failed")?;

        let rsa_signature = rsa::pkcs1v15::Signature::try_from(&decoded_signature[..])
            .change_context(errors::CryptoError::SignatureVerificationFailed)
            .attach_printable("rsa signature transformation failed")?;

        // signature verification
        verifying_key
            .verify(msg, &rsa_signature)
            .map(|_| true)
            .change_context(errors::CryptoError::SignatureVerificationFailed)
            .attach_printable("signature verification step failed")
    }
}

/// TripleDesEde3 hash function
#[derive(Debug)]
#[cfg(feature = "crypto_openssl")]
pub struct TripleDesEde3CBC {
    padding: common_enums::CryptoPadding,
    iv: Vec<u8>,
}

#[cfg(feature = "crypto_openssl")]
impl TripleDesEde3CBC {
    const TRIPLE_DES_KEY_LENGTH: usize = 24;
    /// Initialization Vector (IV) length for TripleDesEde3
    pub const TRIPLE_DES_IV_LENGTH: usize = 8;

    /// Constructor function to be used by the encryptor and decryptor to generate the data type
    pub fn new(
        padding: Option<common_enums::CryptoPadding>,
        iv: Vec<u8>,
    ) -> Result<Self, errors::CryptoError> {
        if iv.len() != Self::TRIPLE_DES_IV_LENGTH {
            Err(errors::CryptoError::InvalidIvLength)?
        };
        let padding = padding.unwrap_or(common_enums::CryptoPadding::PKCS7);
        Ok(Self { iv, padding })
    }
}

#[cfg(feature = "crypto_openssl")]
impl EncodeMessage for TripleDesEde3CBC {
    fn encode_message(
        &self,
        secret: &[u8],
        msg: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        if secret.len() != Self::TRIPLE_DES_KEY_LENGTH {
            Err(errors::CryptoError::InvalidKeyLength)?
        }
        let mut buffer = msg.to_vec();

        if let common_enums::CryptoPadding::ZeroPadding = self.padding {
            let pad_len = Self::TRIPLE_DES_IV_LENGTH - (buffer.len() % Self::TRIPLE_DES_IV_LENGTH);
            if pad_len != Self::TRIPLE_DES_IV_LENGTH {
                buffer.extend(vec![0u8; pad_len]);
            }
        };
        let cipher = openssl::symm::Cipher::des_ede3_cbc();
        openssl::symm::encrypt(cipher, secret, Some(&self.iv), &buffer)
            .change_context(errors::CryptoError::EncodingFailed)
    }
}

/// Generate a random string using a cryptographically secure pseudo-random number generator
/// (CSPRNG). Typically used for generating (readable) keys and passwords.
#[inline]
pub fn generate_cryptographically_secure_random_string(length: usize) -> String {
    use rand::distributions::DistString;

    rand::distributions::Alphanumeric.sample_string(&mut rand::rngs::OsRng, length)
}

/// Generate an array of random bytes using a cryptographically secure pseudo-random number
/// generator (CSPRNG). Typically used for generating keys.
#[inline]
pub fn generate_cryptographically_secure_random_bytes<const N: usize>() -> [u8; N] {
    use rand::RngCore;

    let mut bytes = [0; N];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}

/// A wrapper type to store the encrypted data for sensitive pii domain data types
#[derive(Debug, Clone)]
pub struct Encryptable<T: Clone> {
    inner: T,
    encrypted: Secret<Vec<u8>, EncryptionStrategy>,
}

impl<T: Clone, S: masking::Strategy<T>> Encryptable<Secret<T, S>> {
    /// constructor function to be used by the encryptor and decryptor to generate the data type
    pub fn new(
        masked_data: Secret<T, S>,
        encrypted_data: Secret<Vec<u8>, EncryptionStrategy>,
    ) -> Self {
        Self {
            inner: masked_data,
            encrypted: encrypted_data,
        }
    }
}

impl<T: Clone> Encryptable<T> {
    /// Get the inner data while consuming self
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Get the reference to inner value
    #[inline]
    pub fn get_inner(&self) -> &T {
        &self.inner
    }

    /// Get the inner encrypted data while consuming self
    #[inline]
    pub fn into_encrypted(self) -> Secret<Vec<u8>, EncryptionStrategy> {
        self.encrypted
    }

    /// Deserialize inner value and return new Encryptable object
    pub fn deserialize_inner_value<U, F>(
        self,
        f: F,
    ) -> CustomResult<Encryptable<U>, errors::ParsingError>
    where
        F: FnOnce(T) -> CustomResult<U, errors::ParsingError>,
        U: Clone,
    {
        let inner = self.inner;
        let encrypted = self.encrypted;
        let inner = f(inner)?;
        Ok(Encryptable { inner, encrypted })
    }

    /// consume self and modify the inner value
    pub fn map<U: Clone>(self, f: impl FnOnce(T) -> U) -> Encryptable<U> {
        let encrypted_data = self.encrypted;
        let masked_data = f(self.inner);
        Encryptable {
            inner: masked_data,
            encrypted: encrypted_data,
        }
    }
}

impl<T: Clone> Deref for Encryptable<Secret<T>> {
    type Target = Secret<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Clone> masking::Serialize for Encryptable<T>
where
    T: masking::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<T: Clone> PartialEq for Encryptable<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

/// Type alias for `Option<Encryptable<Secret<String>>>`
pub type OptionalEncryptableSecretString = Option<Encryptable<Secret<String>>>;
/// Type alias for `Option<Encryptable<Secret<String>>>` used for `name` field
pub type OptionalEncryptableName = Option<Encryptable<Secret<String>>>;
/// Type alias for `Option<Encryptable<Secret<String>>>` used for `email` field
pub type OptionalEncryptableEmail = Option<Encryptable<Secret<String, pii::EmailStrategy>>>;
/// Type alias for `Option<Encryptable<Secret<String>>>` used for `phone` field
pub type OptionalEncryptablePhone = Option<Encryptable<Secret<String>>>;
/// Type alias for `Option<Encryptable<Secret<serde_json::Value>>>`
pub type OptionalEncryptableValue = Option<Encryptable<Secret<serde_json::Value>>>;
/// Type alias for `Option<Secret<serde_json::Value>>`
pub type OptionalSecretValue = Option<Secret<serde_json::Value>>;
/// Type alias for `Encryptable<Secret<String>>` used for `name` field
pub type EncryptableName = Encryptable<Secret<String>>;
/// Type alias for `Encryptable<Secret<String>>` used for `email` field
pub type EncryptableEmail = Encryptable<Secret<String, pii::EmailStrategy>>;

/// Represents the RSA-PSS-SHA256 signing algorithm
#[derive(Debug)]
pub struct RsaPssSha256;

impl SignMessage for RsaPssSha256 {
    fn sign_message(
        &self,
        private_key_pem_bytes: &[u8],
        msg_to_sign: &[u8],
    ) -> CustomResult<Vec<u8>, errors::CryptoError> {
        let parsed_pem = pem::parse(private_key_pem_bytes)
            .change_context(errors::CryptoError::EncodingFailed)
            .attach_printable("Failed to parse PEM string")?;
        let key_pair = match parsed_pem.tag() {
            "PRIVATE KEY" => RsaKeyPair::from_pkcs8(parsed_pem.contents())
                .change_context(errors::CryptoError::InvalidKeyLength)
                .attach_printable("Failed to parse PKCS#8 DER with ring"),
            "RSA PRIVATE KEY" => RsaKeyPair::from_der(parsed_pem.contents())
                .change_context(errors::CryptoError::InvalidKeyLength)
                .attach_printable("Failed to parse PKCS#1 DER (using from_der) with ring"),
            tag => Err(errors::CryptoError::InvalidKeyLength).attach_printable(format!(
                "Unexpected PEM tag: {tag}. Expected 'PRIVATE KEY' or 'RSA PRIVATE KEY'",
            )),
        }?;
        let rng = ring_rand::SystemRandom::new();
        let signature_len = key_pair.public().modulus_len();
        let mut signature_bytes = vec![0; signature_len];
        key_pair
            .sign(&RSA_PSS_SHA256, &rng, msg_to_sign, &mut signature_bytes)
            .change_context(errors::CryptoError::EncodingFailed)
            .attach_printable("Failed to sign data with ring")?;

        Ok(signature_bytes)
    }
}

#[cfg(test)]
mod crypto_tests {
    use super::{DecodeMessage, EncodeMessage, SignMessage, VerifySignature};
    use crate::crypto::GenerateDigest;

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
    fn test_sha256_verify_signature() {
        let right_signature =
            hex::decode("123250a72f4e961f31661dbcee0fec0f4714715dc5ae1b573f908a0a5381ddba")
                .expect("Right signature decoding");
        let wrong_signature =
            hex::decode("123250a72f4e961f31661dbcee0fec0f4714715dc5ae1b573f908a0a5381ddbb")
                .expect("Wrong signature decoding");
        let secret = "".as_bytes();
        let data = r#"AJHFH9349JASFJHADJ9834115USD2020-11-13.13:22:34711000000021406655APPROVED12345product_id"#.as_bytes();

        let right_verified = super::Sha256
            .verify_signature(secret, &right_signature, data)
            .expect("Right signature verification result");

        assert!(right_verified);

        let wrong_verified = super::Sha256
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
        let algorithm = super::GcmAes256;

        let encoded_message = algorithm
            .encode_message(&secret, message)
            .expect("Encoded message and tag");

        assert_eq!(
            algorithm
                .decode_message(&secret, encoded_message.into())
                .expect("Decode Failed"),
            message
        );
    }

    #[test]
    fn test_gcm_aes_256_decode_message() {
        // Inputs taken from AES GCM test vectors provided by NIST
        // https://github.com/briansmith/ring/blob/95948b3977013aed16db92ae32e6b8384496a740/tests/aead_aes_256_gcm_tests.txt#L447-L452

        let right_secret =
            hex::decode("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308")
                .expect("Secret decoding");
        let wrong_secret =
            hex::decode("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308309")
                .expect("Secret decoding");
        let message =
            // The three parts of the message are the nonce, ciphertext and tag from the test vector
            hex::decode(
                "cafebabefacedbaddecaf888\
                 522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad\
                 b094dac5d93471bdec1a502270e3cc6c"
            ).expect("Message decoding");

        let algorithm = super::GcmAes256;

        let decoded = algorithm
            .decode_message(&right_secret, message.clone().into())
            .expect("Decoded message");

        assert_eq!(
            decoded,
            hex::decode("d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255")
                .expect("Decoded plaintext message")
        );

        let err_decoded = algorithm.decode_message(&wrong_secret, message.into());

        assert!(err_decoded.is_err());
    }

    #[test]
    fn test_md5_digest() {
        let message = "abcdefghijklmnopqrstuvwxyz".as_bytes();
        assert_eq!(
            format!(
                "{}",
                hex::encode(super::Md5.generate_digest(message).expect("Digest"))
            ),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
    }

    #[test]
    fn test_md5_verify_signature() {
        let right_signature =
            hex::decode("c3fcd3d76192e4007dfb496cca67e13b").expect("signature decoding");
        let wrong_signature =
            hex::decode("d5550730377011948f12cc28889bee590d2a5434d6f54b87562f2dbc2657823f")
                .expect("Wrong signature decoding");
        let secret = "".as_bytes();
        let data = "abcdefghijklmnopqrstuvwxyz".as_bytes();

        let right_verified = super::Md5
            .verify_signature(secret, &right_signature, data)
            .expect("Right signature verification result");

        assert!(right_verified);

        let wrong_verified = super::Md5
            .verify_signature(secret, &wrong_signature, data)
            .expect("Wrong signature verification result");

        assert!(!wrong_verified);
    }

    use ring::signature::{UnparsedPublicKey, RSA_PSS_2048_8192_SHA256};

    #[test]
    fn test_rsa_pss_sha256_verify_signature() {
        let signer = crate::crypto::RsaPssSha256;
        let message = b"abcdefghijklmnopqrstuvwxyz";

        let private_key_pem_bytes =
            std::fs::read("../../private_key.pem").expect("Failed to read private key");
        let parsed_pem = pem::parse(&private_key_pem_bytes).expect("Failed to parse PEM");
        let private_key_der = parsed_pem.contents();

        let signature = signer
            .sign_message(&private_key_pem_bytes, message)
            .expect("Signing failed");

        let key_pair = crate::crypto::RsaKeyPair::from_pkcs8(private_key_der)
            .expect("Failed to parse DER key");
        let public_key_der = key_pair.public().as_ref().to_vec();

        let public_key = UnparsedPublicKey::new(&RSA_PSS_2048_8192_SHA256, &public_key_der);
        assert!(
            public_key.verify(message, &signature).is_ok(),
            "Right signature should verify"
        );

        let mut wrong_signature = signature.clone();
        if let Some(byte) = wrong_signature.first_mut() {
            *byte ^= 0xFF;
        }

        assert!(
            public_key.verify(message, &wrong_signature).is_err(),
            "Wrong signature should not verify"
        );
    }

    #[test]
    fn test_rsasha256_verify_signature() {
        use base64::Engine;
        use rand::rngs::OsRng;
        use rsa::{
            pkcs8::EncodePublicKey,
            signature::{RandomizedSigner, SignatureEncoding},
        };

        use crate::consts::BASE64_ENGINE;

        let mut rng = OsRng;

        let bits = 2048;
        let private_key = rsa::RsaPrivateKey::new(&mut rng, bits).expect("keygen failed");
        let signing_key = rsa::pkcs1v15::SigningKey::<rsa::sha2::Sha256>::new(private_key.clone());

        let message = "{ This is a test message :) }".as_bytes();

        let signature = signing_key.sign_with_rng(&mut rng, message);
        let encoded_signature = BASE64_ENGINE.encode(signature.to_bytes());

        let rsa_public_key = private_key.to_public_key();
        let pem_format_public_key = rsa_public_key
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .expect("transformation failed");
        let encoded_pub_key = BASE64_ENGINE.encode(&pem_format_public_key[..]);

        let right_verified = super::RsaSha256
            .verify_signature(
                encoded_pub_key.as_bytes(),
                encoded_signature.as_bytes(),
                message,
            )
            .expect("Right signature verification result");

        assert!(right_verified);
    }
}
