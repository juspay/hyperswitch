use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use crate::kms::KmsValue;

use super::*;

#[async_trait::async_trait]
/// This trait performs in place decryption of the structure on which this is implemented
pub trait KmsDecrypt {
    /// The output type of the decryption
    type Output;
    /// Decrypts the structure given a KMS client
    async fn decrypt_inner(
        self,
        kms_client: &EncryptionScheme,
    ) -> CustomResult<Self::Output, KmsError>
    where
        Self: Sized;

    // /// Tries to use the Singleton client to decrypt the structure
    // async fn try_decrypt_inner(self) -> CustomResult<Self::Output, KmsError>
    // where
    //     Self: Sized,
    // {
    //     let client =
    //         super::aws_kms::AWS_KMS_CLIENT
    //             .get()
    //             .ok_or(KmsError::KmsClientNotInitialized {
    //                 encryption_scheme: "aws_kms",
    //             })?;
    //     self.decrypt_inner(client).await
    // }
}

#[async_trait::async_trait]
impl KmsDecrypt for &KmsValue {
    type Output = String;
    async fn decrypt_inner(
        self,
        kms_client: &EncryptionScheme,
    ) -> CustomResult<Self::Output, KmsError> {
        kms_client
            .decrypt(self.0.peek().clone())
            .await
            .attach_printable("Failed to decrypt KMS value")
    }
}
