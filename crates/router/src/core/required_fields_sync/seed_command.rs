//! Offline seed command. No registration, inventory, credential, lease or uploader calls.
use std::path::Path;

use super::{
    command::{self, fail, CommandCode, CommandResult, InputProfile, InputSource, Phase, Runtime},
    planner_types, seed,
};
use crate::configs::cli::RequiredFieldsCommand;

pub(super) fn run(
    command: RequiredFieldsCommand,
    runtime: &impl Runtime,
) -> CommandResult<serde_json::Value> {
    let RequiredFieldsCommand::Seed {
        input_profile,
        seed_policy,
        template,
        check,
        output,
    } = command
    else {
        return Err(fail(CommandCode::InputInvalid, Phase::Arguments));
    };
    if check == output.is_some() {
        return Err(fail(CommandCode::InputInvalid, Phase::Arguments));
    }
    if let Some(path) = &output {
        command::output_available(path)?;
    }
    let profile: InputProfile = command::parse(&command::read_file(
        &input_profile,
        command::MAX_PROFILE_BYTES,
        Phase::Input,
    )?)?;
    profile.validate()?;
    // A seed is a named constructor profile, never an ambient Settings/environment projection.
    if !matches!(
        profile.inputs,
        InputSource::Seed {
            stripe_eps_banks: Some(_)
        }
    ) {
        return Err(fail(CommandCode::InputInvalid, Phase::Input));
    }
    let policy: seed::Policy = command::parse(&command::read_file(
        &seed_policy,
        command::MAX_PROFILE_BYTES,
        Phase::Input,
    )?)?;
    let bytes = command::read_file(&template, seed::MAX_SEED_BYTES, Phase::Input)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?;
    let hash = planner_types::digest(&profile).map_err(command::plan_error)?;
    let rendered = runtime.render(
        &profile,
        input_profile.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    let generated =
        seed::generate(text, &rendered.desired, &policy, &hash).map_err(command::plan_error)?;
    if check && generated.text != text {
        return Err(fail(CommandCode::SeedDrift, Phase::Seed));
    }
    if let Some(path) = output {
        command::write_bytes(&path, generated.text.as_bytes())?;
    }
    Ok(
        serde_json::json!({"outcome":if check {"seed_matches"} else {"seed_generated_offline"},
        "report":generated.report,"sdk_verification":"not_executed","uploader":"not_invoked"}),
    )
}
