#[derive(Debug, Clone)]
pub struct NoEncryption;

impl NoEncryption {
    pub fn encrypt(&self, data: String) -> String {
        data
    }

    pub fn decrypt(&self, data: String) -> String {
        data
    }
}
