#![cfg(feature = "deja")]

use router::configs::settings::{DejaMode, DejaSettings};

fn settings_for(mode: DejaMode) -> DejaSettings {
    DejaSettings {
        mode,
        ..DejaSettings::default()
    }
}

#[test]
fn replay_mode_without_source_or_lookup_dir_fails_loudly() {
    let err = router::deja_boot::install(&settings_for(DejaMode::Replay))
        .expect_err("replay mode without source or lookup_dir must abort boot");

    assert!(
        err.contains("deja.replay.source") && err.contains("deja.replay.lookup_dir"),
        "error should mention the missing replay source/lookup_dir, got: {err}"
    );
}

#[test]
fn replay_mode_with_missing_lookup_source_fails_loudly() {
    let missing_source =
        "/__hyperswitch_w3_deja_replay_lookup_table_must_not_exist__/missing.lookup.json";
    let mut settings = settings_for(DejaMode::Replay);
    settings.replay.source = Some(missing_source.to_owned());

    let err = router::deja_boot::install(&settings)
        .expect_err("replay mode with a missing lookup table source must abort boot");

    assert!(
        err.contains("failed to load replay lookup table") && err.contains(missing_source),
        "error should mention lookup-table load failure for {missing_source}, got: {err}"
    );
}

#[test]
fn record_mode_without_kafka_topic_fails_open_with_disabled_report() {
    let report = router::deja_boot::install(&settings_for(DejaMode::Record))
        .expect("record-mode Kafka topic misconfiguration should fail open");

    assert_eq!(report.mode, "disabled");
    assert_eq!(report.run_id, None);
    assert_eq!(
        report.detail.as_deref(),
        Some("record mode requires deja.recording.kafka.topic")
    );
}
