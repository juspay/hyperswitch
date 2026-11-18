//! Early command dispatch. No AppState or unrelated secret/provider initialization.

use std::{collections::HashMap, io::Write as _, path::Path, process::ExitCode};

use api_models::enums::PaymentMethodType;
use payment_methods::configs::settings::{BanksVector, ConnectorBankNames};
use reqwest::header::HeaderValue;

use super::{
    administration::DirectAdminReader,
    command::*,
    planner_types::ProjectionContract,
    raw_json::Json,
    types::{CertifiedReadTarget, CompleteInventory, InventoryLimits},
    EffectiveInputs, Name, RenderInputs,
};
use crate::configs::{
    cli::RequiredFieldsCommand,
    settings::{BankRedirectConfig, RequiredFields, SecuredSecret, Settings},
};

struct Production;
impl super::operation::Runtime for Production {
    async fn run(
        &self,
        invocation: &super::operation::Invocation,
        config: &super::operation::Configuration,
        _: &Path,
    ) -> CommandResult<super::operation::ResultBody> {
        invocation.validate(config)?;
        // These are independent capabilities, never flags in the supplied configuration.
        // There is currently no certified platform provisioning implementation. In particular,
        // do not open a broad router credential or manufacture an in-memory adapter here.
        let target = super::authority::CertifiedExecutionTarget::from_registry(&invocation.target)
            .map_err(|_| fail(CommandCode::CertificationPending, Phase::Target))?;
        if target.registration().target != config.target {
            return Err(fail(CommandCode::TargetMismatch, Phase::Target));
        }
        let _journal = super::journal::CertifiedJournalStore::from_registry(&config.journal_store)
            .map_err(|_| fail(CommandCode::JournalUnavailable, Phase::Status))?;
        if invocation.kind.mutates() {
            let _mutation =
                super::apply_contract::CertifiedMutationTarget::from_registry(&invocation.target)
                    .map_err(|_| fail(CommandCode::UnsupportedServerContract, Phase::Target))?;
            let _image =
                super::operation_executor::ExecutionIdentity::from_registry(&invocation.target)
                    .map_err(|_| fail(CommandCode::NotAuthoritative, Phase::Target))?;
            let _observed = super::operation_executor::ObservedExecutable::capture(
                super::attestation::features(env!("REQUIRED_FIELDS_FEATURE_KEYS"))?,
            )
            .map_err(|_| fail(CommandCode::NotAuthoritative, Phase::Target))?;
            let _verification =
                super::verification::VerificationContract::from_registry(&invocation.target)
                    .map_err(|_| fail(CommandCode::CertificationPending, Phase::Execute))?;
            let _recovery =
                super::recovery::RetainedRecoveryImage::from_registry(&invocation.target)
                    .map_err(|_| fail(CommandCode::CertificationPending, Phase::Execute))?;
        }
        // Activation additionally needs certified storage/controls/collector adapter provisioning.
        // Their shared orchestration is operation_executor::Executor; absent adapters never
        // become a successful "dry" apply. No production registry currently reaches this point.
        Err(fail(CommandCode::CertificationPending, Phase::Execute))
    }
}
impl Runtime for Production {
    fn render(&self, profile: &InputProfile, directory: &Path) -> CommandResult<RenderedInput> {
        let effective = match &profile.inputs {
            InputSource::Seed { stripe_eps_banks } => {
                let banks: BankRedirectConfig = match stripe_eps_banks {
                    None => BankRedirectConfig::default(),
                    Some(values) => {
                        let banks = values
                            .iter()
                            .map(|value| value.as_str().parse())
                            .collect::<Result<_, _>>()
                            .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?;
                        // Retain Some(empty) separately from None without round-tripping through
                        // Settings' CSV parser, which cannot express an empty bank set.
                        BankRedirectConfig(HashMap::from([(
                            PaymentMethodType::Eps,
                            ConnectorBankNames(HashMap::from([(
                                "stripe".to_owned(),
                                BanksVector { banks },
                            )])),
                        )]))
                    }
                };
                EffectiveInputs::capture(&RequiredFields::new(&banks), &banks)
                    .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?
            }
            InputSource::EffectiveSettings {
                settings_file,
                expected_settings_hash,
            } => {
                let path = relative_to(directory, settings_file);
                // Settings historically treats missing files as optional. An explicit profile
                // must not silently fall back to ambient configuration when its file is absent.
                read_file(&path, MAX_PROFILE_BYTES, Phase::Input)?;
                let inputs =
                    Settings::<SecuredSecret>::required_fields_inputs_with_config_path(path)
                        .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?;
                if inputs
                    .settings_hash()
                    .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?
                    != *expected_settings_hash
                {
                    return Err(fail(CommandCode::SettingsMismatch, Phase::Input));
                }
                inputs
            }
        };
        render_effective(profile, &effective)
    }

    fn register(&self, target: &Name, projection: &Name) -> CommandResult<Registration> {
        let read = CertifiedReadTarget::from_registry(target)
            .map_err(|e| inventory_error(e, Phase::Target))?;
        let projection = ProjectionContract::from_registry(projection)
            .map_err(|_| fail(CommandCode::UnsupportedServerContract, Phase::Target))?;
        Ok(Registration { read, projection })
    }
    async fn inventory(
        &self,
        target: &Name,
        credential: &Path,
        limits: InventoryLimits,
    ) -> CommandResult<CompleteInventory> {
        // Registration/config validation has already succeeded. This is an explicit reader file;
        // there is no environment, Settings, secret-provider or writer-token fallback.
        let bytes = read_file(credential, 4096, Phase::Credential)
            .map_err(|_| fail(CommandCode::CredentialInvalid, Phase::Credential))?;
        let token = std::str::from_utf8(&bytes)
            .map_err(|_| fail(CommandCode::CredentialInvalid, Phase::Credential))?;
        let token = token.strip_suffix('\n').unwrap_or(token);
        if token.is_empty()
            || token
                .bytes()
                .any(|v| v.is_ascii_whitespace() || v.is_ascii_control())
        {
            return Err(fail(CommandCode::CredentialInvalid, Phase::Credential));
        }
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| fail(CommandCode::CredentialInvalid, Phase::Credential))?;
        authorization.set_sensitive(true);
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        }
        DirectAdminReader::from_registered(target, authorization)
            .map_err(|e| inventory_error(e, Phase::Credential))?
            .inventory(limits)
            .await
            .map_err(|e| inventory_error(e, Phase::Discovery))
    }
}

/// Deliberately emits only the sanitized public command envelope, never Debug(error_stack).
pub fn reject(code: CommandCode, phase: Phase) -> ExitCode {
    emit_error(fail(code, phase).current_context())
}
fn emit_error(error: &CommandError) -> ExitCode {
    let value = refusal_json(error);
    let _ = writeln!(std::io::stderr().lock(), "{value}");
    ExitCode::from(error.exit_code())
}
pub async fn execute(command: RequiredFieldsCommand) -> ExitCode {
    if matches!(command, RequiredFieldsCommand::Seed { .. }) {
        return match super::seed_command::run(command, &Production) {
            Ok(report) => match writeln!(std::io::stdout().lock(), "{report}") {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => reject(CommandCode::OutputUnavailable, Phase::Output),
            },
            Err(error) => emit_error(error.current_context()),
        };
    }
    if !matches!(
        command,
        RequiredFieldsCommand::Render(_) | RequiredFieldsCommand::Plan { .. }
    ) {
        return match super::operation::run(command, &Production).await {
            Ok((report, exit)) => {
                if writeln!(std::io::stdout().lock(), "{report}").is_err() {
                    ExitCode::from(if exit == 0 { 6 } else { exit })
                } else {
                    ExitCode::from(exit)
                }
            }
            Err(error) => emit_error(error.current_context()),
        };
    }
    match run(command, &Production).await {
        Ok(summary) => match serde_json::to_string(&summary) {
            Ok(text) => {
                if writeln!(std::io::stdout().lock(), "{text}").is_ok() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(_) => reject(CommandCode::OutputUnavailable, Phase::Output),
        },
        Err(error) => emit_error(error.current_context()),
    }
}

/// Observe the already-loaded serving inputs. Never reload ambient settings, decrypt secrets,
/// read a journal, contact the platform, or fail ordinary startup on an attestation refusal.
pub fn startup_attestation(
    conf: &Settings<SecuredSecret>,
) -> CommandResult<super::attestation::ReleaseObservation> {
    let effective = conf
        .required_fields_inputs()
        .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?;
    let mut observation = super::attestation::ReleaseObservation {
        schema_version: 1,
        router_features: super::attestation::features(env!("REQUIRED_FIELDS_FEATURE_KEYS"))?,
        payment_methods_features: super::attestation::features(
            payment_methods::configs::required_fields_renderer::compiled_feature_keys(),
        )?,
        effective_settings: effective
            .settings_hash()
            .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?,
        retained_rules: effective
            .retained_rules_hash()
            .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?,
        profile: None,
        build: None,
        input_manifest: None,
        desired: None,
        bundle: None,
        desired_state: "profile_not_configured",
        image_provenance: "pending_platform_verification",
        application_rollout: "not_asserted",
        synchronization: "not_executed",
        verification: "not_executed",
    };
    if let Some(path) = &conf.required_fields_sync.attestation_profile {
        let desired = (|| {
            let bytes = read_file(path, MAX_PROFILE_BYTES, Phase::Input)?;
            let profile: InputProfile = parse(&bytes)?;
            let InputSource::EffectiveSettings {
                expected_settings_hash,
                ..
            } = &profile.inputs
            else {
                return Err(fail(CommandCode::InputInvalid, Phase::Input));
            };
            if profile.schema_version != 1
                || *expected_settings_hash != observation.effective_settings
            {
                return Err(fail(CommandCode::SettingsMismatch, Phase::Input));
            }
            let rendered = render_effective(&profile, &effective)?;
            let manifest = &rendered.desired.desired.input_manifest;
            observation.profile = Some(super::planner_types::digest(&profile).map_err(plan_error)?);
            observation.build =
                Some(super::planner_types::digest(&manifest.build).map_err(plan_error)?);
            observation.input_manifest = Some(
                super::canonical_hash(super::HashDomain::InputManifest, manifest)
                    .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?,
            );
            observation.desired = Some(
                super::canonical_hash(
                    super::HashDomain::DesiredState,
                    &(
                        super::DESIRED_SCHEMA_VERSION,
                        rendered.desired.desired.contexts.iter().collect::<Vec<_>>(),
                        &rendered.desired.desired.prerequisites,
                    ),
                )
                .map_err(|_| fail(CommandCode::InputInvalid, Phase::Render))?,
            );
            observation.bundle = Some(rendered.desired.bundle_hash().clone());
            Ok::<_, error_stack::Report<CommandError>>(())
        })();
        observation.desired_state = if desired.is_ok() {
            "observed"
        } else {
            "refused"
        };
    }
    Ok(observation)
}

pub fn emit_startup_attestation(conf: &Settings<SecuredSecret>) {
    match startup_attestation(conf).and_then(|v| {
        serde_json::to_string(&v).map_err(|_| fail(CommandCode::InputInvalid, Phase::Output))
    }) {
        Ok(observation) => router_env::logger::info!(
            required_fields_attestation = %observation, "required-fields input observation"),
        Err(_) => router_env::logger::warn!(
            "required-fields input attestation unavailable; serving startup continues"
        ),
    }
}

fn render_effective(
    profile: &InputProfile,
    effective: &EffectiveInputs,
) -> CommandResult<RenderedInput> {
    let build = super::build_identity(profile.release.clone())
        .map_err(|_| fail(CommandCode::UnsupportedBuild, Phase::Render))?;
    let bundle = super::render(RenderInputs {
        effective,
        policy: &profile.policy,
        fields: &profile.catalog,
        exclusions: &profile.exclusions,
        build: &build,
    })
    .map_err(|error| {
        error_stack::Report::from(CommandError {
            code: CommandCode::MappingUnresolved,
            phase: Phase::Render,
            detail: None,
            diagnostic_counts: error
                .current_context()
                .counts
                .iter()
                .map(|(code, count)| (format!("{code:?}"), *count))
                .collect(),
        })
    })?;
    let desired = super::verified_desired(&bundle).map_err(plan_error)?;
    let bytes = bundle
        .canonical_bytes()
        .map_err(|_| fail(CommandCode::InputInvalid, Phase::Render))?;
    let bundle = Json::parse(&bytes).map_err(|_| fail(CommandCode::InputInvalid, Phase::Render))?;
    Ok(RenderedInput { bundle, desired })
}

#[cfg(test)]
mod attestation_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn startup_observes_loaded_inputs_without_claiming_image_rollout_or_sync() {
        let mut conf = Settings::<SecuredSecret>::default();
        conf.required_fields = RequiredFields::new(&conf.bank_config);
        let before = startup_attestation(&conf).unwrap();
        assert_eq!(
            before.effective_settings,
            conf.required_fields_inputs()
                .unwrap()
                .settings_hash()
                .unwrap()
        );
        assert_eq!(
            before.retained_rules,
            conf.required_fields_inputs()
                .unwrap()
                .retained_rules_hash()
                .unwrap()
        );
        assert_eq!(before.desired_state, "profile_not_configured");
        assert!(before.desired.is_none());
        assert_eq!(before.image_provenance, "pending_platform_verification");
        assert_eq!(before.synchronization, "not_executed");
        conf.required_fields_sync.mode = super::super::operation::Mode::Apply;
        conf.required_fields_sync.attestation_profile =
            Some("/nonexistent/synthetic-private-profile".into());
        let after = serde_json::to_string(&startup_attestation(&conf).unwrap()).unwrap();
        assert!(after.contains("refused"));
        assert!(!after.contains("synthetic-private-profile"));
        assert!(!after.contains("master_database"));
        assert!(!after.contains("prefill"));
    }

    #[test]
    #[ignore = "requires registered RF_OPERATION_FIXTURE_ROOT and explicitly excluded synthetic profile"]
    fn startup_desired_observation_uses_loaded_settings_and_refuses_seed_or_hash_drift() {
        let root = std::path::PathBuf::from(std::env::var("RF_OPERATION_FIXTURE_ROOT").unwrap());
        let owner: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
        assert_eq!(owner["purpose"], "Step 13 isolated command checks");
        assert_eq!(owner["root"], serde_json::json!(root));
        assert_eq!(owner["cleanup_scope"], serde_json::json!(root));
        assert!(owner["hosted_target"].is_null());
        let mut profile: InputProfile = parse(
            &read_file(
                &root.join("seed-profile.json"),
                MAX_PROFILE_BYTES,
                Phase::Input,
            )
            .unwrap(),
        )
        .unwrap();
        let work = root.join("startup-attestation");
        std::fs::create_dir(&work).unwrap();
        let mut conf = Settings::<SecuredSecret>::default();
        conf.required_fields = RequiredFields::new(&conf.bank_config);
        let effective = conf.required_fields_inputs().unwrap();
        let expected = effective.settings_hash().unwrap();
        profile.inputs = InputSource::EffectiveSettings {
            settings_file: "not-reloaded.toml".into(),
            expected_settings_hash: expected.clone(),
        };
        let desired = render_effective(&profile, &effective).unwrap();
        for (name, accepted) in [("matching", true), ("drift", false), ("seed", false)] {
            if name == "drift" {
                profile.inputs = InputSource::EffectiveSettings {
                    settings_file: "not-reloaded.toml".into(),
                    expected_settings_hash: format!("sha256:{}", "0".repeat(64))
                        .try_into()
                        .unwrap(),
                };
            }
            if name == "seed" {
                profile.inputs = InputSource::Seed {
                    stripe_eps_banks: None,
                };
            }
            let path = work.join(format!("{name}.json"));
            write_artifact(&path, &serde_json::to_value(&profile).unwrap()).unwrap();
            conf.required_fields_sync.attestation_profile = Some(path);
            let observation = startup_attestation(&conf).unwrap();
            assert_eq!(observation.effective_settings, expected);
            assert_eq!(observation.desired.is_some(), accepted);
            if accepted {
                assert_eq!(
                    observation.bundle.as_ref(),
                    Some(desired.desired.bundle_hash())
                );
            }
            let output = serde_json::to_string(&observation).unwrap();
            assert!(!output.contains("not-reloaded"));
            assert!(!output.contains("stripe"));
            assert_eq!(observation.synchronization, "not_executed");
        }
    }
}
