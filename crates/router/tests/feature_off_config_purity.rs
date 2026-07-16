#![cfg(not(feature = "deja"))]
// Integration test: assertions use panic!/expect(); allow the production-code
// lints the v2 clippy profile denies.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use router::configs::settings::Settings;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn table_header_name(trimmed: &str) -> Option<&str> {
    let table = trimmed.strip_prefix('[')?.strip_suffix(']')?.trim();
    Some(
        table
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .unwrap_or(table)
            .trim(),
    )
}

fn strip_deja_tables(input: &str) -> String {
    let mut output = String::new();
    let mut in_deja_section = false;

    for line in input.lines() {
        if let Some(table) = table_header_name(line.trim()) {
            in_deja_section = table == "deja" || table.starts_with("deja.");
        }
        if in_deja_section {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }

    output
}

const INJECTED_DEJA_TABLES: &str = r#"
[deja]
mode = "record"
run_id = "feature-off-ignored"

[deja.recording]
graph = "disabled"

[deja.recording.kafka]
brokers = ["ignored:9092"]
topic = "ignored"
client_id = "ignored"

[deja.replay]
source = "ignored"
lookup_dir = "ignored"
observed_sink = "ignored"

[deja.sampler]
enabled = true
record_key = "ignored"
timeout_ms = 1
fail_closed = true

[deja.identity]
pod_name_env = "POD_NAME"
git_sha_env = "VERGEN_GIT_SHA"
instance_id = "ignored"
code_sha = "ignored"

[deja.writer]
queue_capacity = 1
batch_size = 1
flush_after_records = 1
flush_interval_ms = 1
shutdown_flush_ms = 1
"#;

fn is_debug_block_close(trimmed: &str) -> bool {
    trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')')
}

fn canonicalize_debug_block(lines: &[&str], index: &mut usize, sort_entries: bool) -> Vec<String> {
    let mut output = Vec::new();
    let mut entries = Vec::<Vec<String>>::new();

    while let Some(line) = lines.get(*index) {
        let line = (*line).to_owned();
        let trimmed = line.trim();
        let is_block_close = is_debug_block_close(trimmed);
        let opens_child_block =
            trimmed.ends_with('{') || trimmed.ends_with('[') || trimmed.ends_with('(');
        // Sort the entries of any `{ ... }` block (maps/sets are unordered, and
        // sorting a struct's fields is harmless because it is applied to both
        // sides). Newtype `( ... )` and array `[ ... ]` blocks preserve order.
        // This makes the comparison independent of HashMap iteration order,
        // which otherwise leaks through fields like `supported_payment_methods`.
        let child_sorts_map_entries = trimmed.ends_with('{');
        *index += 1;

        if is_block_close {
            if sort_entries {
                entries.sort();
                output.extend(entries.into_iter().flatten());
            }
            output.push(line);
            return output;
        }

        let chunk = if opens_child_block {
            let mut chunk = vec![line];
            chunk.extend(canonicalize_debug_block(
                lines,
                index,
                child_sorts_map_entries,
            ));
            chunk
        } else {
            vec![line]
        };

        if sort_entries {
            entries.push(chunk);
        } else {
            output.extend(chunk);
        }
    }

    if sort_entries {
        entries.sort();
        output.extend(entries.into_iter().flatten());
    }
    output
}

fn canonicalize_settings_debug(debug: &str) -> String {
    let lines = debug.lines().collect::<Vec<_>>();
    let mut index = 0;
    canonicalize_debug_block(&lines, &mut index, false).join("\n")
}

fn settings_debug(path: PathBuf) -> String {
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("required config file {path:?} must be readable: {err}"));

    std::env::set_var("RUN_ENV", "development");
    for key in [
        "ROUTER__DEJA__MODE",
        "ROUTER__DEJA__RUN_ID",
        "ROUTER__DEJA__RECORDING__KAFKA__BROKERS",
        "ROUTER__DEJA__RECORDING__KAFKA__TOPIC",
        "ROUTER__DEJA__REPLAY__SOURCE",
        "ROUTER__DEJA__REPLAY__LOOKUP_DIR",
        "ROUTER__DEJA__REPLAY__OBSERVED_SINK",
        "ROUTER__DEJA__SAMPLER__ENABLED",
        "ROUTER__DEJA__WRITER__QUEUE_CAPACITY",
    ] {
        std::env::remove_var(key);
    }
    let settings = Settings::with_config_path(Some(path)).expect("settings deserialize");
    canonicalize_settings_debug(&format!("{settings:#?}"))
}

fn with_injected_deja_tables(base: &str) -> String {
    let mut output = base.to_owned();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(INJECTED_DEJA_TABLES);
    output
}

struct CleanupDir {
    path: PathBuf,
}

impl CleanupDir {
    fn new() -> Self {
        let unique_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();

        for _ in 0..100 {
            let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "feature_off_config_purity-{}-{unique_nanos}-{unique}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create temp config dir {path:?}: {err}"),
            }
        }

        panic!("create unique temp config dir after 100 attempts");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CleanupDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn feature_off_config_bytes_ignore_deja_surfaces() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("repo root")
        .to_path_buf();
    let real_parseable_configs = ["config/development.toml", "config/docker_compose.toml"];
    let temp = CleanupDir::new();

    for rel in real_parseable_configs {
        let original = fs::read_to_string(repo.join(rel)).expect("read config");
        let without_deja = strip_deja_tables(&original);
        let with_deja = with_injected_deja_tables(&without_deja);
        let without_path = temp.path().join(rel.replace('/', "__") + "__without_deja");
        let with_path = temp.path().join(rel.replace('/', "__") + "__with_deja");
        fs::write(&without_path, without_deja).expect("write config without deja");
        fs::write(&with_path, with_deja).expect("write config with injected deja");

        assert_eq!(
            settings_debug(with_path),
            settings_debug(without_path),
            "feature-off Settings shape changed after injecting only [deja] tables into {rel}"
        );
    }

    let synthetic_without = temp.path().join("synthetic_minimal_without_deja.toml");
    let synthetic_with = temp.path().join("synthetic_minimal_with_deja.toml");
    fs::write(&synthetic_without, "\n").expect("write synthetic config without deja");
    fs::write(&synthetic_with, with_injected_deja_tables("\n"))
        .expect("write synthetic config with injected deja");
    assert_eq!(
        settings_debug(synthetic_with),
        settings_debug(synthetic_without),
        "feature-off Settings shape changed after injecting [deja] tables into a synthetic minimal config"
    );
}
