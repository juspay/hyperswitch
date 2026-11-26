//! Fail the build when a connector config TOML is malformed.
//!
//! `ConnectorConfig::new` embeds one of `toml/{development,sandbox,production}.toml`
//! with `include_str!` and parses it at RUNTIME, choosing the file by cargo
//! feature. Two consequences, and together they are why a dropped array header
//! reached production:
//!
//! 1. A malformed file compiles. `include_str!` only reads bytes; nothing checks
//!    that they are valid TOML, so `cargo check` and `cargo build` stay green.
//! 2. `new` parses the whole file in one `toml::from_str` and both public entry
//!    points propagate the error with `?`. So one bad stanza does not disable one
//!    connector — it disables `get_connector_config` for EVERY connector, in that
//!    environment only.
//!
//! A build script runs under `cargo check`, which CI already does for the
//! `release` feature, so this turns that runtime failure into a build failure for
//! all three files at once — including the ones the active feature does not select.
//!
//! Scope is deliberately well-formedness, not the typed schema: `ConnectorConfig`
//! lives in the crate being built and cannot be used from here. The typed parse is
//! covered by `every_environment_config_parses` in `src/connector.rs`.

use std::path::Path;

const CONFIG_FILES: [&str; 3] = ["development.toml", "sandbox.toml", "production.toml"];

fn main() {
    let toml_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("toml");

    for file in CONFIG_FILES {
        let path = toml_dir.join(file);
        println!("cargo:rerun-if-changed={}", path.display());

        let contents = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) => fail(&format!("cannot read {}: {err}", path.display())),
        };

        if let Err(err) = contents.parse::<toml::Table>() {
            fail(&format!(
                "{} is not valid TOML, so ConnectorConfig::new would fail for EVERY \
                 connector in that environment: {err}",
                path.display()
            ));
        }
    }
}

/// Stop the build with a readable reason.
///
/// `cargo::error=` renders as a build error rather than a build-script panic and
/// backtrace, and a non-zero exit is what actually fails the build. Deliberately
/// not `panic!`: `clippy::panic` is denied in this workspace, and the panic
/// output buries the message under a stack trace.
fn fail(message: &str) -> ! {
    println!("cargo::error={message}");
    std::process::exit(1)
}
