//! Records and replays every file-storage call at the trait boundary.
//!
//! One decorator over `Arc<dyn FileStorageInterface>` covers every backend, the
//! ones that exist and the ones added later, because the only place a client is
//! built (`FileStorageConfig::get_file_storage_client`) wraps it.
//!
//! Identity is the file key, plus a digest and the length of the bytes an upload
//! sends, and for a completed multipart upload the sorted part numbers. The
//! digest stands in for the payload because args are rendered in full and a
//! stored file has no bounded size — a size limit, not a secrecy one; returned
//! values go on the tape whole. Multipart upload ids are per-run handles, so
//! they take no part in an identity. A miss answers with the call's own failure,
//! which the caller already handles.

use common_utils::errors::CustomResult;

use crate::file_storage::{FileStorageError, FileStorageInterface};

#[derive(Clone)]
pub(super) struct RecordingFileStorage {
    inner: std::sync::Arc<dyn FileStorageInterface>,
}

impl RecordingFileStorage {
    pub(super) fn new(inner: std::sync::Arc<dyn FileStorageInterface>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl FileStorageInterface for RecordingFileStorage {
    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "upload_file",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<(), FileStorageError>,
        args = serde_json::json!({
            "file_key": file_key,
            "len": file.len(),
            "sha256": payload_digest(&file),
        }),
        on_miss = Err(FileStorageError::UploadFailed.into()),
    )]
    async fn upload_file(
        &self,
        file_key: &str,
        file: Vec<u8>,
    ) -> CustomResult<(), FileStorageError> {
        self.inner.upload_file(file_key, file).await
    }

    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "delete_file",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<(), FileStorageError>,
        args = serde_json::json!({ "file_key": file_key }),
        on_miss = Err(FileStorageError::DeleteFailed.into()),
    )]
    async fn delete_file(&self, file_key: &str) -> CustomResult<(), FileStorageError> {
        self.inner.delete_file(file_key).await
    }

    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "retrieve_file",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<Vec<u8>, FileStorageError>,
        args = serde_json::json!({ "file_key": file_key }),
        on_miss = Err(FileStorageError::RetrieveFailed.into()),
    )]
    async fn retrieve_file(&self, file_key: &str) -> CustomResult<Vec<u8>, FileStorageError> {
        self.inner.retrieve_file(file_key).await
    }

    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "create_multipart_upload",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<String, FileStorageError>,
        args = serde_json::json!({ "file_key": file_key }),
        on_miss = Err(FileStorageError::MultipartCreateFailed.into()),
    )]
    async fn create_multipart_upload(
        &self,
        file_key: &str,
    ) -> CustomResult<String, FileStorageError> {
        self.inner.create_multipart_upload(file_key).await
    }

    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "upload_part",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<String, FileStorageError>,
        args = serde_json::json!({
            "file_key": file_key,
            "part_number": part_number,
            "len": body.len(),
            "sha256": payload_digest(&body),
        }),
        on_miss = Err(FileStorageError::MultipartUploadPartFailed.into()),
    )]
    async fn upload_part(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        body: Vec<u8>,
    ) -> CustomResult<String, FileStorageError> {
        self.inner
            .upload_part(file_key, upload_id, part_number, body)
            .await
    }

    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "complete_multipart_upload",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<Option<String>, FileStorageError>,
        args = serde_json::json!({
            "file_key": file_key,
            "part_numbers": sorted_part_numbers(&parts),
        }),
        on_miss = Err(FileStorageError::MultipartCompleteFailed.into()),
    )]
    async fn complete_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> CustomResult<Option<String>, FileStorageError> {
        self.inner
            .complete_multipart_upload(file_key, upload_id, parts)
            .await
    }

    #[deja::boundary(
        boundary = "file_storage",
        component = "external_services::file_storage",
        operation = "abort_multipart_upload",
        replay = Substitute,
        effect = Http,
        future = "boxed",
        codec = deja::codec::ResultCodec::<(), FileStorageError>,
        args = serde_json::json!({ "file_key": file_key }),
        on_miss = Err(FileStorageError::MultipartAbortFailed.into()),
    )]
    async fn abort_multipart_upload(
        &self,
        file_key: &str,
        upload_id: &str,
    ) -> CustomResult<(), FileStorageError> {
        self.inner.abort_multipart_upload(file_key, upload_id).await
    }

    // Not a boundary: signing is local computation, so no call leaves the
    // process for a replay to intercept, and the signature it returns is over
    // the deployment's own credentials, which differ from the recording's.
    async fn get_presigned_download_url(
        &self,
        file_key: &str,
        expires_in: std::time::Duration,
        download_file_name: Option<&str>,
    ) -> CustomResult<url::Url, FileStorageError> {
        self.inner
            .get_presigned_download_url(file_key, expires_in, download_file_name)
            .await
    }
}

/// A SHA-256 of an upload's bytes, hex encoded, to stand for them in the call's
/// identity: args are rendered in full and a stored file has no bounded size,
/// and two uploads under one key are two calls, which a length cannot tell
/// apart.
fn payload_digest(bytes: &[u8]) -> String {
    use common_utils::crypto::{GenerateDigest, Sha256};

    // Infallible in practice; the marker keeps args bounded rather than putting
    // the file itself in them.
    Sha256
        .generate_digest(bytes)
        .map_or_else(|_| String::from("sha256-unavailable"), hex::encode)
}

fn sorted_part_numbers(parts: &[(i32, String)]) -> Vec<i32> {
    let mut numbers: Vec<i32> = parts.iter().map(|(number, _)| *number).collect();
    numbers.sort_unstable();
    numbers
}

#[cfg(all(test, feature = "deja"))]
mod tests {
    use super::FileStorageError;

    /// The declarations as written, with this test module cut off so its own
    /// text cannot satisfy an anchor.
    fn declarations() -> &'static str {
        include_str!("recording.rs")
            .rsplit_once("#[cfg(all(test, feature = \"deja\"))]")
            .map_or("", |(head, _)| head)
    }

    /// The attribute for `operation`, ending at the closing paren at the
    /// attribute's own indent so a comment inside it cannot end it early.
    #[allow(
        clippy::panic,
        reason = "test helper: a free fn, so allow-panic-in-tests does not cover it; a missing declaration should fail the test loudly"
    )]
    fn attribute(operation: &str) -> String {
        let anchor = format!("operation = \"{operation}\",");
        let (before, after) = declarations()
            .split_once(&anchor)
            .unwrap_or_else(|| panic!("no seam declares operation `{operation}`"));
        let opening = before
            .rsplit_once("#[deja::boundary(")
            .map_or("", |(_, opening)| opening);
        let rest = after
            .split_once("\n    )]\n")
            .map(|(rest, _)| rest)
            .unwrap_or_else(|| panic!("seam `{operation}` has no closing `)]`"));
        format!("{opening}{anchor}{rest}")
    }

    const SEAMS: [(&str, &str, &str); 7] = [
        ("upload_file", "()", "UploadFailed"),
        ("delete_file", "()", "DeleteFailed"),
        ("retrieve_file", "Vec<u8>", "RetrieveFailed"),
        ("create_multipart_upload", "String", "MultipartCreateFailed"),
        ("upload_part", "String", "MultipartUploadPartFailed"),
        (
            "complete_multipart_upload",
            "Option<String>",
            "MultipartCompleteFailed",
        ),
        ("abort_multipart_upload", "()", "MultipartAbortFailed"),
    ];

    #[test]
    fn every_storage_call_is_a_substituted_seam_with_its_own_miss() {
        for (operation, ok, miss) in SEAMS {
            let attribute = attribute(operation);
            assert!(
                attribute.contains("replay = Substitute"),
                "{operation} must substitute"
            );
            assert!(
                attribute.contains("component = \"external_services::file_storage\""),
                "{operation} lost its component"
            );
            assert!(
                attribute.contains("future = \"boxed\""),
                "{operation} sits under async_trait and needs the boxed shape"
            );
            assert!(
                attribute.contains(&format!("ResultCodec::<{ok}, FileStorageError>")),
                "{operation} must reconstruct a recorded error as well as a value"
            );
            assert!(
                attribute.contains(&format!("on_miss = Err(FileStorageError::{miss}.into())")),
                "{operation} must answer a miss with its own failure, never a value"
            );
        }
    }

    #[test]
    fn operations_cannot_alias() {
        let mut seen = std::collections::BTreeSet::new();
        for (operation, _, _) in SEAMS {
            assert!(seen.insert(operation), "{operation} declared twice");
            assert_eq!(
                declarations()
                    .matches(&format!("operation = \"{operation}\","))
                    .count(),
                1,
                "{operation} must be declared exactly once"
            );
        }
    }

    #[test]
    fn identity_keys_every_call_and_carries_no_per_run_handle() {
        for (operation, _, _) in SEAMS {
            let attribute = attribute(operation);
            let args = attribute.split_once("args =").map_or("", |(_, args)| args);
            let args = args.split_once("on_miss").map_or(args, |(args, _)| args);
            assert!(args.contains("\"file_key\""), "{operation} lost its key");
            assert!(
                !args.contains("upload_id"),
                "{operation} identity must not carry a handle the backend mints per run: {args}"
            );
        }
    }

    /// The two calls that carry bytes must digest them. Without this a candidate
    /// that uploads different content under the same key reaches the same
    /// address, and the replay reports no divergence at all.
    #[test]
    fn an_upload_is_identified_by_what_it_sends() {
        for (operation, payload) in [("upload_file", "file"), ("upload_part", "body")] {
            assert!(
                attribute(operation).contains(&format!("payload_digest(&{payload})")),
                "{operation} must tell two different payloads under one key apart"
            );
        }
    }

    #[test]
    fn the_digest_separates_what_a_length_cannot() {
        assert_eq!(super::payload_digest(b"one"), super::payload_digest(b"one"));
        assert_ne!(super::payload_digest(b"one"), super::payload_digest(b"two"));
    }

    #[test]
    fn the_factory_wraps_whatever_backend_it_builds() {
        let (_, factory) = include_str!("../file_storage.rs")
            .split_once("pub async fn get_file_storage_client")
            .expect("factory present");
        let factory = factory.split_once("\n    }\n").expect("factory end").0;
        assert!(
            factory.contains("recording::RecordingFileStorage::new(client)"),
            "the single factory must wrap its result"
        );
        assert!(
            factory.contains("#[cfg(feature = \"deja\")]"),
            "the wrapper must be absent from a build without deja"
        );
    }

    #[test]
    fn a_recorded_error_round_trips() {
        let recorded = serde_json::to_value(FileStorageError::RetrieveFailed).expect("serialize");
        let replayed: FileStorageError = serde_json::from_value(recorded).expect("deserialize");
        assert_eq!(replayed, FileStorageError::RetrieveFailed);
    }
}
