//! Opt-in offline profile inspection, not the router's future sync command.
//! Reads bounded profile/identity TOML inputs and creates one new output file. No AppState/client.

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    process::ExitCode,
};

use payment_methods::configs::{
    required_fields_renderer::{
        canonical_bytes, compiled_feature_keys, parse_contract, render, ArtifactIdentity,
        BuildIdentity, Contract, EffectiveInputs, ExclusionPolicy, FieldContractCatalog,
        InputError, InputResult, MappingPolicy, Name, ReleaseIdentity, RenderInputs,
        MAX_CONTRACT_BYTES, SCHEMA_VERSION,
    },
    settings::{BankRedirectConfig, RequiredFields},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    schema_version: u16,
    profile_id: Name,
    bank_config: BankRedirectConfig,
    policy: MappingPolicy,
    catalog: FieldContractCatalog,
    exclusions: ExclusionPolicy,
}

impl Contract for Profile {
    fn validate(&self) -> InputResult<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(InputError::InputInvalid.into());
        }
        self.policy.validate()?;
        self.catalog.validate()?;
        self.exclusions.validate()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfflineRelease {
    release: ReleaseIdentity,
}

impl Contract for OfflineRelease {
    fn validate(&self) -> InputResult<()> {
        if !matches!(self.release.artifact, ArtifactIdentity::Offline) {
            return Err(InputError::InputInvalid.into());
        }
        Ok(())
    }
}

fn read<T: Contract>(path: std::ffi::OsString) -> Result<T, Box<dyn std::error::Error>> {
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(u64::try_from(MAX_CONTRACT_BYTES + 1)?)
        .read_to_string(&mut text)?;
    Ok(parse_contract(&text)?)
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = args
        .next()
        .ok_or("expected PROFILE.toml RELEASE.toml OUTPUT.json")?;
    let release = args
        .next()
        .ok_or("expected PROFILE.toml RELEASE.toml OUTPUT.json")?;
    let output = args
        .next()
        .ok_or("expected PROFILE.toml RELEASE.toml OUTPUT.json")?;
    if args.next().is_some() {
        return Err("expected exactly three paths".into());
    }
    let profile: Profile = read(input)?;
    let release: OfflineRelease = read(release)?;
    let build = BuildIdentity::for_offline_renderer(release.release, compiled_feature_keys())?;
    let rules = RequiredFields::new(&profile.bank_config);
    let effective = EffectiveInputs::capture(&rules, &profile.bank_config)?;
    let inputs = RenderInputs {
        effective: &effective,
        policy: &profile.policy,
        fields: &profile.catalog,
        exclusions: &profile.exclusions,
        build: &build,
    };
    let manifest = inputs.manifest()?;
    let (result, exit) = match render(inputs) {
        Ok(bundle) => (
            json!({"status":"rendered", "bundle_hash":bundle.hash()?, "desired_hash":bundle.desired_hash()?, "bundle":bundle}),
            ExitCode::SUCCESS,
        ),
        Err(error) => (
            json!({"status":"blocked", "diagnostics":error.current_context(), "input_manifest":manifest}),
            ExitCode::from(2),
        ),
    };
    let report = json!({"schema_version":1, "profile_id":profile.profile_id, "hosted_apply":"disabled", "payment_methods_feature_keys":compiled_feature_keys(), "result":result});
    // Refuse replacement, including a pre-existing symlink. Never print input text on failure.
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    output.write_all(&canonical_bytes(&report)?)?;
    Ok(exit)
}

#[allow(
    clippy::print_stderr,
    reason = "offline example deliberately does not initialize a logger"
)]
fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(_) => {
            eprintln!("offline profile inspection failed: check bounded input contract and unused output path");
            ExitCode::FAILURE
        }
    }
}
