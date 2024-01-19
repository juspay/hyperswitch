//!
//! Module for local file system storage operations
//!

use std::{
    fs::{remove_file, File},
    io::{Read, Write},
    path::PathBuf,
};

use error_stack::{IntoReport, ResultExt};

use common_utils::errors::CustomResult;

/// Constructs the file path for a given file key within the file system.
/// The file path is generated based on the workspace path and the provided file key.
fn get_file_path(file_key: impl AsRef<str>) -> PathBuf {
    let mut file_path = PathBuf::new();
    #[cfg(feature = "logs")]
    file_path.push(router_env::env::workspace_path());
    #[cfg(not(feature = "logs"))]
    file_path.push(std::env::current_dir().unwrap_or(".".into()));

    file_path.push("files");
    file_path.push(file_key.as_ref());
    file_path
}

/// Represents a file system for storing and managing files locally.
#[derive(Debug, Clone)]
pub struct FileSystem;

impl FileSystem {
    /// Saves the provided file data to the file system under the specified file key.
    pub(super) fn save_file_to_fs(
        &self,
        file_key: impl AsRef<str>,
        file_data: Vec<u8>,
    ) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(&file_key);

        // Ignore the file name and create directories in the `file_path` if not exists
        std::fs::create_dir_all(
            file_path
                .parent()
                .ok_or(FileSystemStorageError::CreateDirFailed)
                .into_report()
                .attach_printable("Failed to obtain parent directory")?,
        )
        .into_report()
        .change_context(FileSystemStorageError::CreateDirFailed)?;

        let mut file = File::create(file_path)
            .into_report()
            .change_context(FileSystemStorageError::CreateFailure)?;
        file.write_all(&file_data)
            .into_report()
            .change_context(FileSystemStorageError::WriteFailure)?;
        Ok(())
    }

    /// Deletes the file associated with the specified file key from the file system.
    pub(super) fn delete_file_from_fs(
        &self,
        file_key: impl AsRef<str>,
    ) -> CustomResult<(), FileSystemStorageError> {
        let file_path = get_file_path(file_key);
        remove_file(file_path)
            .into_report()
            .change_context(FileSystemStorageError::DeleteFailure)?;
        Ok(())
    }

    /// Retrieves the file content associated with the specified file key from the file system.
    pub(super) fn retrieve_file_from_fs(
        &self,
        file_key: impl AsRef<str>,
    ) -> CustomResult<Vec<u8>, FileSystemStorageError> {
        let mut received_data: Vec<u8> = Vec::new();
        let file_path = get_file_path(file_key);
        let mut file = File::open(file_path)
            .into_report()
            .change_context(FileSystemStorageError::FileOpenFailure)?;
        file.read_to_end(&mut received_data)
            .into_report()
            .change_context(FileSystemStorageError::ReadFailure)?;
        Ok(received_data)
    }
}

/// Represents an error that can occur during local file system storage operations.
#[derive(Debug, thiserror::Error)]
pub enum FileSystemStorageError {
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
