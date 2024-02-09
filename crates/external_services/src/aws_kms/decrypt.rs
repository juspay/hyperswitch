use common_utils::errors::CustomResult;

use super::*;

#[async_trait::async_trait]
/// This trait performs in place decryption of the structure on which this is implemented
pub trait AwsKmsDecrypt {
    /// The output type of the decryption
    type Output;
    /// Decrypts the structure given a AWS KMS client
    async fn decrypt_inner(
        self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError>
    where
        Self: Sized;

    /// Tries to use the Singleton client to decrypt the structure
    async fn try_decrypt_inner(self) -> CustomResult<Self::Output, AwsKmsError>
    where
        Self: Sized,
    {
        let client = AWS_KMS_CLIENT
            .get()
            .ok_or(AwsKmsError::AwsKmsClientNotInitialized)?;
        self.decrypt_inner(client).await
    }
}

#[async_trait::async_trait]
impl AwsKmsDecrypt for &AwsKmsValue {
    type Output = String;
    async fn decrypt_inner(
        self,
        aws_kms_client: &AwsKmsClient,
    ) -> CustomResult<Self::Output, AwsKmsError> {
        aws_kms_client
            .decrypt(self.0.peek())
            .await
            .attach_printable("Failed to decrypt AWS KMS value")
    }
}
