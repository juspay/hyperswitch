//! The shipped catalogue, loaded the way the service loads it.
//!
//! Every other test builds its own definitions. This one reads `config/observability.toml` — the
//! file a deployment actually runs — and checks that it deserializes, validates, and resolves into
//! the shape the port was supposed to produce.
//!
//! That matters more than it looks. The catalogue is a hand-checked snapshot of somebody else's
//! source of truth, and the ways it can go wrong are all silent: a threshold that lost a digit
//! still evaluates, a definition whose dimensions did not survive the copy queries a metric that
//! reports nothing and simply never fires, and a severity pointing at an unconfigured destination
//! evaluates perfectly and delivers nowhere. None of those is a compile error. The counts and
//! spot-checks below are what turn them into a failing test.
//!
//! When the rendering is automated and this file stops being a snapshot, these assertions are the
//! ones the generator has to keep satisfying.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;

use observability::{
    domain::alarm::{catalogue::Catalogue, MissingDataPolicy},
    settings::Settings,
};

/// The config directory, from the crate this test lives in.
fn config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config")
        .join("observability.toml")
}

fn catalogue() -> Catalogue {
    let settings = Settings::with_config_path(Some(config_path()))
        .expect("config/observability.toml should deserialize");

    // The whole boot check, not just the CloudWatch section: this is also what proves every
    // severity's destination and the failure destination exist in `chat.destinations`.
    settings
        .validate()
        .expect("config/observability.toml should validate");

    Catalogue::resolve(&settings.cloudwatch).expect("the catalogue should resolve")
}

/// The port's headline shape: twenty `rds-alerts` definitions carrying fifty-three severities, and
/// twenty CloudWatch queries rather than fifty-three — severities of one definition differ only by
/// threshold, and a threshold is not part of a query.
#[test]
fn the_shipped_catalogue_is_the_twenty_rds_definitions() {
    let catalogue = catalogue();

    assert_eq!(catalogue.definitions(), 20, "definitions");
    assert_eq!(catalogue.targets.len(), 53, "severities");
    assert_eq!(catalogue.readings.len(), 20, "distinct queries");

    assert!(
        catalogue
            .targets
            .iter()
            .all(|target| target.classification == "rds-alerts"),
        "this ticket ships `rds-alerts` only"
    );
}

/// Both periods the subset uses, in the proportion it uses them. The split is what decides how
/// many requests a run makes: the twelve minute metrics share one grid for both windows, the eight
/// five-minute ones need two.
#[test]
fn the_catalogue_carries_both_periods() {
    let catalogue = catalogue();

    let seconds = catalogue
        .readings
        .iter()
        .map(|reading| reading.period.seconds())
        .collect::<Vec<_>>();

    assert_eq!(seconds.iter().filter(|period| **period == 60).count(), 12);
    assert_eq!(seconds.iter().filter(|period| **period == 300).count(), 8);
}

/// Every definition names a stream. A definition that lost its dimensions in the copy would query
/// the undimensioned metric, which reports nothing — an alarm that never fires and never says why.
#[test]
fn every_reading_names_a_dimensioned_stream() {
    for reading in catalogue().readings {
        assert!(
            !reading.labels.is_empty(),
            "{} has no dimensions",
            reading.describe()
        );
        assert_eq!(reading.namespace, "AWS/RDS");
    }
}

/// The dimension values the snapshot resolved, checked against the infrastructure they were copied
/// from. These are the identifiers that decide *which database* is being watched, so a typo here
/// is a catalogue that watches nothing.
#[test]
fn the_resolved_dimension_values_are_the_ones_terraform_publishes() {
    let catalogue = catalogue();

    let values = catalogue
        .readings
        .iter()
        .flat_map(|reading| {
            reading
                .labels
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for expected in [
        "DBInstanceIdentifier=hyperswitchdb-primary",
        "DBInstanceIdentifier=failover-replica-1",
        "DBClusterIdentifier=hyperswitchdb-cluster",
        "Role=WRITER",
        "Role=READER",
    ] {
        assert!(
            values.iter().any(|value| value == expected),
            "expected {expected} among {values:?}"
        );
    }
}

/// The missing-data policies the subset actually uses, which is the fact the ticket got wrong.
///
/// Only the two CPU `sev1` entries treat a gap as breaching; every other severity treats it as
/// within the threshold, because that is the Terraform module's default rather than AWS's
/// `missing`. Neither `missing` nor `ignore` appears — so `INSUFFICIENT_DATA` is unreachable for
/// this catalogue, and nothing here needs the previous state that `ignore` would demand.
#[test]
fn only_two_severities_treat_missing_data_as_breaching() {
    let catalogue = catalogue();

    let breaching = catalogue
        .targets
        .iter()
        .filter(|target| target.rule.treat_missing_data == MissingDataPolicy::Breaching)
        .map(|target| target.name())
        .collect::<Vec<_>>();

    assert_eq!(
        breaching,
        vec!["rds-failover-cpu/sev1", "rds-primary-cpu/sev1"]
    );

    assert!(
        !catalogue
            .targets
            .iter()
            .any(|target| target.rule.treat_missing_data == MissingDataPolicy::Missing),
        "no rds definition uses `missing`, so INSUFFICIENT_DATA cannot arise from the policy"
    );
}

/// `datapoints_to_alarm` is used seven times in the full seventy-six-definition catalogue and not
/// once in `rds-alerts`, so every severity here is N-out-of-N. M-out-of-N is implemented and
/// tested, but nothing shipped exercises it — worth asserting so the day a rendered catalogue does
/// start using it, this test is what notices.
#[test]
fn no_shipped_severity_uses_m_out_of_n() {
    for target in catalogue().targets {
        assert_eq!(
            target.rule.datapoints_to_alarm,
            target.rule.evaluation_periods,
            "{}",
            target.name()
        );
    }
}

/// The evaluation windows in use, spot-checked. Values outside this set mean a transcription slip.
#[test]
fn the_evaluation_windows_are_the_ones_the_subset_uses() {
    for target in catalogue().targets {
        assert!(
            [1, 2, 3, 5].contains(&target.rule.evaluation_periods),
            "{} has an unexpected evaluation_periods {}",
            target.name(),
            target.rule.evaluation_periods
        );
    }
}

/// One definition, end to end, against the HCL it was copied from. A threshold that lost a digit
/// deserializes and evaluates perfectly well; only comparing it to the source catches it.
#[test]
fn a_spot_checked_definition_matches_its_source() {
    let catalogue = catalogue();

    let target = catalogue
        .targets
        .iter()
        .find(|target| target.name() == "rds-failover-freeable-memory/sev1")
        .expect("the definition should be in the catalogue");

    // `threshold = 1717986918`, `LessThanOrEqualToThreshold`, `evaluation_periods = 5`, and no
    // `treat_missing_data` — so the module's `notBreaching` default applies.
    assert_eq!(target.rule.threshold, 1_717_986_918.0);
    assert_eq!(target.rule.evaluation_periods, 5);
    assert_eq!(
        target.rule.treat_missing_data,
        MissingDataPolicy::NotBreaching
    );
    assert!(target
        .description
        .starts_with("SEV1: RDS failover database freeable memory"));

    let reading = &catalogue.readings[target.reading];
    assert_eq!(reading.metric_name, "FreeableMemory");
    assert_eq!(reading.period.seconds(), 60);
}

/// The snapshot has to say it is one. The flag is what makes the service warn at every boot, and a
/// rendered catalogue is what clears it — so this assertion is expected to be inverted, not
/// deleted, when [#23444] lands.
#[test]
fn the_shipped_catalogue_declares_itself_a_snapshot() {
    let settings = Settings::with_config_path(Some(config_path()))
        .expect("config/observability.toml should deserialize");
    let source = &settings.cloudwatch.source;

    assert!(source.temporary, "the snapshot must declare itself");
    assert!(!source.origin.is_empty(), "a snapshot must say where from");
    assert_eq!(source.revision.len(), 40, "a full git revision");
}
