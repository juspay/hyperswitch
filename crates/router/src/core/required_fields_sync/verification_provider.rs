//! Non-serving startup using the same pinned FileDataSource/LocalResolutionProvider as serving.
//! No AppState, HTTP client, writer credentials, global OpenFeature registration or poller.
use std::path::Path;

use superposition_provider::{
    data_source::file::FileDataSource, EvaluationContext, LocalResolutionProvider, RefreshStrategy,
    SuperpositionDataSource,
};

use super::{apply_contract as ac, raw_json::Json, verification::bytes_hash, Digest};

pub struct StartupReceipt {
    pub(super) content: Digest,
    pub(super) config: Digest,
}
pub async fn startup_canary(
    backup: &Path,
    unavailable_primary: &Path,
    expected_file: &Digest,
    expected_export: &[u8],
) -> ac::ApplyResult<StartupReceipt> {
    // The caller provisions an isolated absent primary path, not an unavailable live endpoint.
    if unavailable_primary.try_exists().unwrap_or(true)
        || backup == unavailable_primary
        || expected_export.len() > 4 * 1024 * 1024
    {
        return Err(ac::fail(ac::ApplyCode::VerificationFailed));
    }
    let metadata = ac::checked(std::fs::symlink_metadata(backup))?;
    if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 {
        return Err(ac::fail(ac::ApplyCode::VerificationFailed));
    }
    let before = ac::checked(std::fs::read(backup))?;
    if &bytes_hash(&before)? != expected_file {
        return Err(ac::fail(ac::ApplyCode::VerificationFailed));
    }
    let expected_json = ac::checked(Json::parse(expected_export))?;
    let expected: superposition_types::Config =
        ac::checked(serde_json::from_slice(expected_export))?;
    // Do not accept missing fields hidden by permissive defaults in the provider parser.
    for field in ["contexts", "overrides", "default_configs", "dimensions"] {
        ac::checked(expected_json.get(field))?;
    }
    let provider = LocalResolutionProvider::new(
        Box::new(ac::checked(FileDataSource::new(
            unavailable_primary.to_path_buf(),
        ))?),
        Some(Box::new(ac::checked(FileDataSource::new(
            backup.to_path_buf(),
        ))?)),
        RefreshStrategy::Manual,
    );
    let result = async {
        ac::checked(provider.init(EvaluationContext::default()).await)?;
        let actual = ac::checked(provider.fetch_config(None).await)?
            .into_data()
            .ok_or_else(|| ac::fail(ac::ApplyCode::VerificationFailed))?;
        let parsed = |v: &superposition_types::Config| -> ac::ApplyResult<Vec<u8>> {
            ac::checked(ac::checked(Json::parse(&ac::checked(serde_json::to_vec(v))?))?.canonical())
        };
        if parsed(&actual.data)? != parsed(&expected)?
            || before != ac::checked(std::fs::read(backup))?
            || unavailable_primary.try_exists().unwrap_or(true)
        {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        Ok(StartupReceipt {
            content: bytes_hash(&before)?,
            config: bytes_hash(&ac::checked(expected_json.canonical())?)?,
        })
    }
    .await;
    ac::checked(provider.close_provider().await)?;
    result
}
