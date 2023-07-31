use common_utils::errors::CustomResult;

use super::*;

#[async_trait::async_trait]
/// This trait performs in place decryption of the structure on which this is implemented
pub trait KmsDecrypt {
    /// The output type of the decryption
    type Output;
    /// Decrypts the structure given a KMS client
    async fn decrypt_inner(self, kms_client: &KmsClient) -> CustomResult<Self::Output, KmsError>
    where
        Self: Sized;

    /// Tries to use the Singleton client to decrypt the structure
    async fn try_decrypt_inner(self) -> CustomResult<Self::Output, KmsError>
    where
        Self: Sized,
    {
        let client = KMS_CLIENT.get().ok_or(KmsError::KmsClientNotInitialized)?;
        self.decrypt_inner(client).await
    }
}

#[async_trait::async_trait]
impl KmsDecrypt for &KMSValue {
    type Output = String;
    async fn decrypt_inner(self, kms_client: &KmsClient) -> CustomResult<Self::Output, KmsError> {
        kms_client
            .decrypt(self.0.peek())
            .await
            .attach_printable("Failed to decrypt KMS value")
    }
}
