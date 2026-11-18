//! Production command adapters. No AppState, serving provider, mutation client or lease.

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
        let build = super::build_identity(profile.release.clone())
            .map_err(|_| fail(CommandCode::UnsupportedBuild, Phase::Render))?;
        let bundle = super::render(RenderInputs {
            effective: &effective,
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
        let bundle =
            Json::parse(&bytes).map_err(|_| fail(CommandCode::InputInvalid, Phase::Render))?;
        Ok(RenderedInput { bundle, desired })
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
    ExitCode::from(1)
}
pub async fn execute(command: RequiredFieldsCommand) -> ExitCode {
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
