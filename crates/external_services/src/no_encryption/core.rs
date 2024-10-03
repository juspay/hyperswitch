//! No encryption core functionalities

/// No encryption type
#[derive(Debug, Clone)]
pub struct NoEncryption;

impl NoEncryption {
    /// Encryption functionality
    pub fn encrypt(&self, data: impl AsRef<[u8]>) -> Vec<u8> {
        data.as_ref().into()
    }

    /// Decryption functionality
    pub fn decrypt(&self, data: impl AsRef<[u8]>) -> Vec<u8> {
        data.as_ref().into()
    }
}
