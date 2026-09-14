//! Module for local file system storage operations

use std::{
    fs::{remove_file, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

use common_utils::errors::CustomResult;
use error_stack::ResultExt;

use crate::file_storage::{FileStorageError, FileStorageInterface};

/// Constructs the file path for a given file key within the file system.
/// The file path is generated based on the workspace path and the provided file key.
fn get_file_path(file_key: impl AsRef<str>) -> PathBuf {
    let mut file_path = PathBuf::new();
    file_path.push(std::env::current_dir().unwrap_or(".".into()));

    file_path.push("files");
    file_path.push(file_key.as_ref());
    file_path
}

/// Represents a file system for storing and managing files locally.
#[derive(Debug, Clone)]
pub(super) struct FileSystem;

impl FileSystem {
    /// Saves the provided file data to the file system under the specified file key.
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(file_key);

        // Ignore the file name and create directories in the `file_path` if not exists
        std::fs::create_dir_all(
            file_path
                .parent()
                .ok_or(FileSystemStorageError::CreateDirFailed)
                .attach_printable("Failed to obtain parent directory")?,
        )
        .change_context(FileSystemStorageError::CreateDirFailed)?;

        let mut file_handler =
            File::create(file_path).change_context(FileSystemStorageError::CreateFailure)?;
        file_handler
            .write_all(&file)
            .change_context(FileSystemStorageError::WriteFailure)?;
        Ok(())
    }

    /// Deletes the file associated with the specified file key from the file system.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(file_key);
        remove_file(file_path).change_context(FileSystemStorageError::DeleteFailure)?;
        Ok(())
    }

    /// Retrieves the file content associated with the specified file key from the file system.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileSystemStorageError> {
        let mut received_data: Vec<u8> = Vec::new();
        let file_path = get_file_path(file_key);
        let mut file =
            File::open(file_path).change_context(FileSystemStorageError::FileOpenFailure)?;
        file.read_to_end(&mut received_data)
            .change_context(FileSystemStorageError::ReadFailure)?;
        Ok(received_data)
    }

    /// Truncates the staging file that parts are appended to.
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, FileSystemStorageError> {
        let file_path = get_file_path(staging_key(file_key));

        std::fs::create_dir_all(
            file_path
                .parent()
                .ok_or(FileSystemStorageError::CreateDirFailed)
                .attach_printable("Failed to obtain parent directory")?,
        )
        .change_context(FileSystemStorageError::CreateDirFailed)?;

        File::create(file_path).change_context(FileSystemStorageError::CreateFailure)?;
        Ok(file_key.to_owned())
    }

    /// Appends a part to the staging file.
    async fn upload_part(
        &self,
        file_key: &str,
        body: Vec<u8>,
    ) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(staging_key(file_key));
        let mut file_handler = OpenOptions::new()
            .append(true)
            .open(file_path)
            .change_context(FileSystemStorageError::FileOpenFailure)?;
        file_handler
            .write_all(&body)
            .change_context(FileSystemStorageError::WriteFailure)?;
        Ok(())
    }

    /// Moves the completed staging file into place under the real key.
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<(), FileSystemStorageError> {
        std::fs::rename(
            get_file_path(staging_key(file_key)),
            get_file_path(file_key),
        )
        .change_context(FileSystemStorageError::WriteFailure)?;
        Ok(())
    }

    /// Removes the staging file for an abandoned upload.
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<(), FileSystemStorageError> {
        remove_file(get_file_path(staging_key(file_key)))
            .change_context(FileSystemStorageError::DeleteFailure)?;
        Ok(())
    }
}

/// Key of the staging file that parts are appended to before the upload is completed.
fn staging_key(file_key: &str) -> String {
    format!("{file_key}.part")
}

#[async_trait::async_trait]
impl FileStorageInterface for FileSystem {
    /// Saves the provided file data to the file system under the specified file key.
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError> {
        self.upload_file(file_key, file)
            .await
            .change_context(FileStorageError::UploadFailed)?;
        Ok(())
    }

    /// Deletes the file associated with the specified file key from the file system.
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError> {
        self.delete_file(file_key)
            .await
            .change_context(FileStorageError::DeleteFailed)?;
        Ok(())
    }

    /// Retrieves the file content associated with the specified file key from the file system.
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError> {
        Ok(self
            .retrieve_file(file_key)
            .await
            .change_context(FileStorageError::RetrieveFailed)?)
    }

    /// Truncates the staging file, returning the file key as the upload identifier.
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, FileStorageError> {
        Ok(self
            .create_multipart_upload(file_key)
            .await
            .change_context(FileStorageError::MultipartCreateFailed)?)
    }

    /// Appends a part to the staging file. Part numbers are ignored, since appends arrive in order.
    async fn upload_part(
        &self,
        file_key: &str,
        _upload_id: &str,
        _part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, FileStorageError> {
        self.upload_part(file_key, body)
            .await
            .change_context(FileStorageError::MultipartUploadPartFailed)?;
        Ok(String::new())
    }

    /// Moves the staging file into place under the real key. No lifecycle rule applies locally.
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        _upload_id: &str,
        _parts: Vec<(i32, String)>,
    ) -> CustomResult<Option<String>, FileStorageError> {
        self.complete_multipart_upload(file_key)
            .await
            .change_context(FileStorageError::MultipartCompleteFailed)?;
        Ok(None)
    }

    /// Removes the staging file for an abandoned upload.
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
        _upload_id: &str,
    ) -> CustomResult<(), FileStorageError> {
        self.abort_multipart_upload(file_key)
            .await
            .change_context(FileStorageError::MultipartAbortFailed)?;
        Ok(())
    }

    /// A local path is neither signed nor reachable by a merchant.
    async fn get_presigned_download_url(
        &self,
        _file_key: &str,
        _expires_in: std::time::Duration,
        _download_file_name: Option<&str>,
    ) -> CustomResult<url::Url, FileStorageError> {
        Err(error_stack::report!(
            FileStorageError::PresigningNotSupported
        ))
        .attach_printable("The file_system storage backend cannot issue presigned URLs")
    }
}

/// Represents an error that can occur during local file system storage operations.
#[derive(Debug, thiserror::Error)]
enum FileSystemStorageError {
    /// Error indicating opening a file failed
    #[error("Failed while opening the file")]
    FileOpenFailure,

    /// Error indicating file creation failed.
    #[error("Failed to create file")]
    CreateFailure,

    /// Error indicating reading a file failed.
    #[error("Failed while reading the file")]
    ReadFailure,

    /// Error indicating writing to a file failed.
    #[error("Failed while writing into file")]
    WriteFailure,

    /// Error indicating file deletion failed.
    #[error("Failed while deleting the file")]
    DeleteFailure,

    /// Error indicating directory creation failed
    #[error("Failed while creating a directory")]
    CreateDirFailed,
}
