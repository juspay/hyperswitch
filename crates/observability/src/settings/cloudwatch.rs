//! The metric alarm catalogue, as configuration.
//!
//! ## Who owns this
//!
//! **The infrastructure repository does.** Terraform already knows every resource identifier and
//! already expands the templated classifications; it renders the catalogue and Helm delivers it as
//! configuration, and this service validates and evaluates what arrives. There is deliberately no
//! Terraform parsing, no resource discovery and no group expansion in Rust, because a second
//! implementation of "which cache nodes exist" would drift from the first the day a node is added.
//!
//! That decision is what shapes the schema below. A definition carries **its dimensions, resolved**
//! — `[{ name = "DBInstanceIdentifier", value = "hyperswitchdb-primary" }]`, not a key naming a map
//! somewhere else and not a group to expand. A CloudWatch metric is identified by its dimension
//! combination, so the producer that knows the identifiers is the right place for them to be named,
//! and the consumer's job is to check that what arrived is usable. See [`Dimension`] for why they
//! are name/value pairs rather than the map they look like they should be.
//!
//! ## Field names are the catalogue's own
//!
//! Everything a severity carries — `threshold`, `comparison_operator`, `description`,
//! `evaluation_periods`, `datapoints_to_alarm`, `treat_missing_data` — keeps the name and the
//! spelling `classified_metric_alarms` gives it, and so do the **defaults**. A severity that omits
//! `evaluation_periods`, `statistic`, `comparison_operator` or `treat_missing_data` in the HCL may
//! omit it here and mean the same thing, so a rendered entry is checkable against its source by
//! eye. Those defaults are the Terraform module's rather than AWS's, and the two differ where it
//! matters most: AWS defaults `treat_missing_data` to `missing`, `classified_metric_alarms`
//! defaults it to `notBreaching`, and it is the module's value the deployed alarms actually have.
//!
//! ## Snapshots say so
//!
//! Until the rendering is automated, the catalogue in `config/observability.toml` is a hand-checked
//! snapshot. [`CatalogueSource`] is how it says that: an origin, the revision it was taken from,
//! and a `temporary` flag that makes the service log a warning at every boot. A comment would have
//! done the same job until someone deleted it.
//!
//! ## Why the section can be absent
//!
//! `[cloudwatch]` with no alarms is off, and validated as such. A deployment that only forwards
//! alerts — everything before this ticket — must not have to name an AWS region to boot.

use std::collections::BTreeMap;

pub use external_services::metrics_service::aws_cloudwatch::CloudWatchConfig;
use serde::Deserialize;

use crate::errors::ConfigurationError;

/// CloudWatch periods are one of the five high-resolution values or any positive multiple of a
/// minute. Checked at boot rather than left to the provider, which would reject the whole batch on
/// the first evaluation and take every other metric down with it.
const HIGH_RESOLUTION_PERIODS: [u32; 5] = [1, 5, 10, 20, 30];

/// The default number of extra datapoints fetched beyond the evaluation window.
///
/// See [`AlarmRule::evaluation_range`](crate::domain::alarm::AlarmRule::evaluation_range) for why
/// this is configuration with a default rather than a constant: AWS documents that its own widening
/// varies with the period and publishes no formula, and 2 is the only number its worked examples
/// put to a standard-resolution metric.
const DEFAULT_MISSING_DATA_LOOKBACK: u32 = 2;

/// The Terraform module's default evaluation window, reproduced so an omission ports faithfully.
const DEFAULT_EVALUATION_PERIODS: u32 = 5;

/// The Terraform module's default period, likewise.
const DEFAULT_PERIOD: u32 = 60;

fn default_missing_data_lookback() -> u32 {
    DEFAULT_MISSING_DATA_LOOKBACK
}

fn default_evaluation_periods() -> u32 {
    DEFAULT_EVALUATION_PERIODS
}

fn default_period() -> u32 {
    DEFAULT_PERIOD
}

/// Everything the evaluator reads from configuration.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CloudWatchSettings {
    /// How to reach CloudWatch. `external_services`' own type, so the region is validated exactly
    /// where the client validates it and cannot drift into a second spelling.
    #[serde(flatten)]
    pub client: CloudWatchConfig,

    /// How many datapoints beyond the window to retrieve, so that gaps in the recent periods can
    /// be covered by real readings from further back.
    #[serde(default = "default_missing_data_lookback")]
    pub missing_data_lookback: u32,

    /// How far to hold back from the latest completed minute before evaluating.
    ///
    /// Zero matches CloudWatch, which evaluates through the last completed minute and does not
    /// wait for stragglers. Raise it where a metric's ingestion lag is known to exceed a minute:
    /// a datapoint that lands after its window was evaluated cannot retroactively produce the
    /// transition it would have caused, because nothing here remembers the decision.
    pub evaluation_delay_seconds: u32,

    /// The chat destination each severity's announcements go to, by severity id.
    ///
    /// Three ids all naming one channel in sandbox. Routing is configuration precisely so that
    /// splitting them later is not a code change.
    pub severity_destinations: BTreeMap<String, String>,

    /// Where a failed *evaluation* is reported — a CloudWatch outage rather than a breached
    /// threshold. Kept separate from the severity destinations because it is an operational
    /// message about this service, not an alert about the estate.
    pub failure_destination: Option<String>,

    /// Where the catalogue below came from, and whether it is a hand-made snapshot.
    pub source: CatalogueSource,

    /// The catalogue, by definition name. Empty means the evaluator is off.
    ///
    /// A `BTreeMap` rather than a `HashMap` so that a run's results come back in a stable order —
    /// the endpoint's first caller is a human comparing two curl outputs.
    pub alarms: BTreeMap<String, AlarmDefinition>,
}

/// The provenance of the catalogue, carried in the configuration that carries the catalogue.
///
/// Renders as a boot log line, and `temporary` renders as a warning. The point is that a snapshot
/// announces itself for as long as it exists: a hand-checked copy of somebody else's source of
/// truth is fine to develop against and dangerous to forget about, and the version that replaces it
/// fills in the same three fields from the generator rather than needing a new shape.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CatalogueSource {
    /// Where the definitions came from — a repository and path, or the renderer that produced them.
    pub origin: String,

    /// The revision of that source, so an entry can be checked against the thing it was copied
    /// from rather than against whatever that file says today.
    pub revision: String,

    /// Whether this is a hand-made snapshot rather than rendered output.
    pub temporary: bool,
}

/// One catalogue entry: a metric, and the severities watching it.
#[derive(Debug, Deserialize, Clone)]
pub struct AlarmDefinition {
    /// The group this definition belongs to, carried through from the HCL. Not used to decide
    /// anything here; it is what says which Terraform-side ticket owns the entry.
    pub classification: String,

    /// The metric's name, as CloudWatch publishes it.
    pub metric_name: String,

    /// The namespace it is published under.
    pub namespace: String,

    /// The dimensions narrowing it to one reporting stream, already resolved.
    ///
    /// **Resolved, and never empty.** A CloudWatch metric is *identified* by its dimension
    /// combination: a query carrying the wrong ones — or none — reads a different metric, one that
    /// reports nothing and therefore never fires. The producer resolves them because it is the
    /// side that knows the identifiers; this side refuses an entry that arrived without any.
    pub dimensions: Vec<Dimension>,

    /// How long one datapoint covers, in seconds.
    #[serde(default = "default_period")]
    pub period: u32,

    /// How the values inside a period are reduced.
    #[serde(default)]
    pub statistic: Statistic,

    /// The severities, by id. Each is an independent generation rule: they share readings and
    /// neither suppresses the other.
    pub severities: BTreeMap<String, SeverityRule>,
}

/// One severity's threshold, window and message.
#[derive(Debug, Deserialize, Clone)]
pub struct SeverityRule {
    /// The number readings are compared against.
    pub threshold: f64,

    /// Which side of the threshold breaches.
    #[serde(default)]
    pub comparison_operator: ComparisonOperator,

    /// The message an operator reads. Hand-written in the catalogue and reproduced verbatim — it
    /// is the alert, not a label for it.
    pub description: String,

    /// N: how many of the most recent periods form the window.
    #[serde(default = "default_evaluation_periods")]
    pub evaluation_periods: u32,

    /// M: how many of the window's readings must breach. Defaults to N, as CloudWatch does.
    pub datapoints_to_alarm: Option<u32>,

    /// What a period with no reading counts as.
    #[serde(default)]
    pub treat_missing_data: TreatMissingData,
}

/// One CloudWatch dimension: a name and a value.
///
/// **A pair, not a map entry, and that is not a style preference.** The `config` crate lowercases
/// every key it reads — from files as well as from the environment — so a dimension written as
/// `{ DBInstanceIdentifier = "…" }` arrives as `dbinstanceidentifier`. CloudWatch dimension names
/// are case-sensitive, and a query carrying the wrong case names a metric that does not exist:
/// it returns no datapoints, so under this catalogue's `notBreaching` default every alarm on it
/// evaluates perfectly and never fires. Silence is the one failure mode an alarm cannot afford.
///
/// [`ChatSettings::destinations`](super::ChatSettings::destinations) meets the same lowercasing
/// and answers it by refusing ids that would not survive the round trip. A dimension name cannot
/// be spelled to survive it, so it moves out of the key instead — values keep their case. This
/// also happens to be the shape CloudWatch itself uses on the wire.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Dimension {
    /// The dimension's name, case preserved — `DBInstanceIdentifier`, `Role`.
    pub name: String,
    /// The value identifying one reporting stream.
    pub value: String,
}

/// How the values inside one period are reduced, spelled as CloudWatch spells it.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum Statistic {
    /// Arithmetic mean.
    #[default]
    Average,
    /// Largest value in the period.
    Maximum,
    /// Smallest value in the period.
    Minimum,
    /// Every value in the period added together.
    Sum,
}

/// Which side of the threshold breaches, spelled as CloudWatch spells it.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComparisonOperator {
    /// Breaches above the threshold. The Terraform module's default.
    #[default]
    GreaterThanThreshold,
    /// Breaches at or above the threshold.
    GreaterThanOrEqualToThreshold,
    /// Breaches below the threshold.
    LessThanThreshold,
    /// Breaches at or below the threshold.
    LessThanOrEqualToThreshold,
}

/// What a period with no reading counts as, spelled as the HCL spells it.
///
/// All four of CloudWatch's, including the one this service cannot honour, so that a catalogue
/// asking for it is *refused* rather than quietly evaluated as something else. See
/// [`crate::domain::alarm`].
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TreatMissingData {
    /// A gap counts as a breach.
    Breaching,
    /// A gap counts as within the threshold. The Terraform module's default.
    #[default]
    NotBreaching,
    /// A window of nothing but gaps is `INSUFFICIENT_DATA`.
    Missing,
    /// Keep whatever state the alarm already had. Rejected at boot: this evaluator has no memory
    /// of a previous state to keep.
    Ignore,
}

impl CloudWatchSettings {
    /// Whether any definition is configured.
    ///
    /// The off switch, and there is no separate flag: a catalogue with nothing in it has nothing
    /// to evaluate, so neither a region nor a destination is demanded of a deployment that only
    /// forwards alerts.
    pub fn is_enabled(&self) -> bool {
        !self.alarms.is_empty()
    }

    /// Every severity id any definition uses.
    pub fn severities_in_use(&self) -> impl Iterator<Item = &str> {
        self.alarms
            .values()
            .flat_map(|definition| definition.severities.keys().map(String::as_str))
    }

    /// Reject a catalogue that cannot be evaluated.
    ///
    /// Everything checkable without a network call is checked here, because the alternative is
    /// discovering it on the first evaluation — by which time the failure is a chat message about
    /// a metric nobody can find rather than a service that refused to start.
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        if !self.is_enabled() {
            return Ok(());
        }

        self.client
            .validate()
            .map_err(|reason| ConfigurationError::ConfigParsingError(reason.to_owned()))?;

        if self.failure_destination.is_none() {
            Err(ConfigurationError::ConfigParsingError(
                "cloudwatch.failure_destination must name a chat destination when alarms are \
                 configured, so a CloudWatch outage is reported somewhere"
                    .into(),
            ))?
        }

        for (name, definition) in &self.alarms {
            definition.validate(name)?;

            for severity in definition.severities.keys() {
                if !self.severity_destinations.contains_key(severity) {
                    Err(ConfigurationError::ConfigParsingError(format!(
                        "alarm `{name}` uses severity `{severity}`, which has no entry in \
                         cloudwatch.severity_destinations"
                    )))?
                }
            }
        }

        Ok(())
    }
}

impl AlarmDefinition {
    fn validate(&self, name: &str) -> Result<(), ConfigurationError> {
        let reject = |reason: String| ConfigurationError::ConfigParsingError(reason);

        if !is_cloudwatch_period(self.period) {
            Err(reject(format!(
                "alarm `{name}` has period {}, which is not a CloudWatch period: use 1, 5, 10, \
                 20, 30, or a multiple of 60",
                self.period
            )))?
        }

        // An empty dimension set does not mean "every stream" — it names the metric published with
        // no dimensions at all, which for every metric in this catalogue reports nothing. An entry
        // that arrived without dimensions is a rendering that went wrong upstream, and it would
        // otherwise present as an alarm that simply never fires.
        if self.dimensions.is_empty() {
            Err(reject(format!(
                "alarm `{name}` has no dimensions, which would read the undimensioned metric \
                 rather than the stream it names"
            )))?
        }

        if self
            .dimensions
            .iter()
            .any(|dimension| dimension.name.trim().is_empty() || dimension.value.trim().is_empty())
        {
            Err(reject(format!(
                "alarm `{name}` has a dimension with an empty name or value, which usually means \
                 an unresolved reference reached the rendered catalogue"
            )))?
        }

        // Two entries for one dimension name is a rendering that went wrong; one of them would win
        // silently and select a stream nobody asked for.
        let mut names = self
            .dimensions
            .iter()
            .map(|dimension| dimension.name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        if names.len() != total {
            Err(reject(format!(
                "alarm `{name}` names the same dimension twice"
            )))?
        }

        if self.severities.is_empty() {
            Err(reject(format!("alarm `{name}` has no severities")))?
        }

        for (severity, rule) in &self.severities {
            rule.validate(name, severity)?;
        }

        Ok(())
    }
}

impl SeverityRule {
    /// M, defaulted to N the way CloudWatch defaults it.
    pub fn datapoints_to_alarm(&self) -> u32 {
        self.datapoints_to_alarm.unwrap_or(self.evaluation_periods)
    }

    fn validate(&self, alarm: &str, severity: &str) -> Result<(), ConfigurationError> {
        let reject = |reason: String| ConfigurationError::ConfigParsingError(reason);
        let where_ = format!("alarm `{alarm}` severity `{severity}`");

        if self.evaluation_periods == 0 {
            Err(reject(format!("{where_} has evaluation_periods 0")))?
        }

        // M above N can never be satisfied, and M of zero is satisfied by nothing at all: both are
        // an alarm that silently never fires, which is the worst way for a threshold to be wrong.
        let to_alarm = self.datapoints_to_alarm();
        if to_alarm == 0 || to_alarm > self.evaluation_periods {
            Err(reject(format!(
                "{where_} has datapoints_to_alarm {to_alarm}, which must be between 1 and its \
                 evaluation_periods {}",
                self.evaluation_periods
            )))?
        }

        if self.description.trim().is_empty() {
            Err(reject(format!("{where_} has no description to announce")))?
        }

        if self.treat_missing_data == TreatMissingData::Ignore {
            Err(reject(format!(
                "{where_} asks for treat_missing_data `ignore`, which keeps the alarm's previous \
                 state. This evaluator reconstructs both windows from CloudWatch on every request \
                 and remembers no previous state, so it cannot reproduce it — use `missing`, \
                 `breaching` or `notBreaching`"
            )))?
        }

        Ok(())
    }
}

fn is_cloudwatch_period(seconds: u32) -> bool {
    HIGH_RESOLUTION_PERIODS.contains(&seconds) || (seconds > 0 && seconds % 60 == 0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn settings_from(value: serde_json::Value) -> CloudWatchSettings {
        serde_json::from_value(value).unwrap()
    }

    /// The shape a rendered catalogue produces: dimensions resolved, everything else spelled as
    /// the HCL spells it.
    fn minimal() -> serde_json::Value {
        serde_json::json!({
            "region": "ap-south-1",
            "failure_destination": "smoke",
            "severity_destinations": { "sev1": "smoke" },
            "alarms": {
                "rds-primary-cpu": {
                    "classification": "rds-alerts",
                    "metric_name": "CPUUtilization",
                    "namespace": "AWS/RDS",
                    "dimensions": [{ "name": "DBInstanceIdentifier", "value": "hyperswitchdb-primary" }],
                    "period": 60,
                    "statistic": "Average",
                    "severities": {
                        "sev1": {
                            "threshold": 90.0,
                            "comparison_operator": "GreaterThanOrEqualToThreshold",
                            "description": "SEV1: CPU is high",
                            "evaluation_periods": 1,
                            "treat_missing_data": "breaching",
                        },
                    },
                },
            },
        })
    }

    /// The property that keeps a rendered entry checkable by eye: what the HCL leaves out, this
    /// may leave out, and the values that appear are the Terraform module's own defaults.
    #[test]
    fn omitted_fields_take_the_terraform_modules_defaults() {
        let settings = settings_from(serde_json::json!({
            "region": "ap-south-1",
            "failure_destination": "smoke",
            "severity_destinations": { "sev1": "smoke" },
            "alarms": {
                "bare": {
                    "classification": "rds-alerts",
                    "metric_name": "CPUUtilization",
                    "namespace": "AWS/RDS",
                    "dimensions": [{ "name": "DBClusterIdentifier", "value": "cluster" }],
                    "severities": {
                        "sev1": { "threshold": 1.0, "description": "d" },
                    },
                },
            },
        }));

        let definition = &settings.alarms["bare"];
        assert_eq!(definition.period, 60);
        assert_eq!(definition.statistic, Statistic::Average);

        let rule = &definition.severities["sev1"];
        assert_eq!(rule.evaluation_periods, 5);
        assert_eq!(
            rule.comparison_operator,
            ComparisonOperator::GreaterThanThreshold
        );
        // Not AWS's `missing`. The deployed alarms have what the module put there.
        assert_eq!(rule.treat_missing_data, TreatMissingData::NotBreaching);
        // CloudWatch's own default for M, which the module leaves to AWS by sending nothing.
        assert_eq!(rule.datapoints_to_alarm(), 5);

        settings.validate().unwrap();
    }

    #[test]
    fn a_catalogue_with_no_alarms_needs_no_region_or_destination() {
        CloudWatchSettings::default().validate().unwrap();
        assert!(!CloudWatchSettings::default().is_enabled());
    }

    #[test]
    fn a_configured_catalogue_needs_a_region() {
        let mut settings = settings_from(minimal());
        settings.client.region = String::new();

        assert!(settings.validate().is_err());
    }

    /// A definition with no dimensions would read the undimensioned stream — a different metric,
    /// which reports nothing and would never fire.
    #[test]
    fn a_definition_without_dimensions_fails_the_boot() {
        let mut settings = settings_from(minimal());
        settings
            .alarms
            .get_mut("rds-primary-cpu")
            .unwrap()
            .dimensions
            .clear();

        let error = settings.validate().unwrap_err().to_string();
        assert!(error.contains("rds-primary-cpu"), "{error}");
    }

    /// The shape an unresolved Terraform reference arrives in once it has been rendered to a
    /// string: present, and empty.
    #[test]
    fn a_dimension_with_an_empty_value_fails_the_boot() {
        let mut settings = settings_from(minimal());
        settings
            .alarms
            .get_mut("rds-primary-cpu")
            .unwrap()
            .dimensions
            .push(Dimension {
                name: "Role".to_owned(),
                value: String::new(),
            });

        assert!(settings.validate().is_err());
    }

    #[test]
    fn the_same_dimension_named_twice_fails_the_boot() {
        let mut settings = settings_from(minimal());
        let definition = settings.alarms.get_mut("rds-primary-cpu").unwrap();
        definition.dimensions.push(Dimension {
            name: "DBInstanceIdentifier".to_owned(),
            value: "somewhere-else".to_owned(),
        });

        assert!(settings.validate().is_err());
    }

    /// The bug this shape exists to prevent. `config` lowercases every key it reads, so a
    /// dimension written as a map arrives as `dbinstanceidentifier` — a case CloudWatch does not
    /// know, naming a metric that reports nothing, so every alarm on it would evaluate cleanly and
    /// never fire. Carried as a value, the name survives.
    #[test]
    fn a_dimension_name_keeps_its_case_through_configuration() {
        let settings = settings_from(minimal());
        let dimensions = &settings.alarms["rds-primary-cpu"].dimensions;

        assert_eq!(dimensions.len(), 1);
        assert_eq!(dimensions[0].name, "DBInstanceIdentifier");
        assert_eq!(dimensions[0].value, "hyperswitchdb-primary");
    }

    #[test]
    fn a_severity_with_nowhere_to_go_fails_the_boot() {
        let mut settings = settings_from(minimal());
        settings.severity_destinations.clear();

        let error = settings.validate().unwrap_err().to_string();
        assert!(error.contains("sev1"), "{error}");
    }

    #[test]
    fn a_catalogue_with_no_failure_destination_fails_the_boot() {
        let mut settings = settings_from(minimal());
        settings.failure_destination = None;

        assert!(settings.validate().is_err());
    }

    /// The parity limitation, made loud. `ignore` needs a memory of the previous state, and there
    /// is none — so it is refused rather than silently evaluated as something else.
    #[test]
    fn treat_missing_data_ignore_is_refused_with_its_reason() {
        let mut settings = settings_from(minimal());
        settings
            .alarms
            .get_mut("rds-primary-cpu")
            .unwrap()
            .severities
            .get_mut("sev1")
            .unwrap()
            .treat_missing_data = TreatMissingData::Ignore;

        let error = settings.validate().unwrap_err().to_string();
        assert!(error.contains("previous state"), "{error}");
    }

    /// Both of these are an alarm that can never fire, which is worse than one that fires wrongly.
    #[test]
    fn an_unsatisfiable_m_out_of_n_fails_the_boot() {
        for (window, to_alarm) in [(3_u32, 4_u32), (3, 0)] {
            let mut settings = settings_from(minimal());
            let rule = settings
                .alarms
                .get_mut("rds-primary-cpu")
                .unwrap()
                .severities
                .get_mut("sev1")
                .unwrap();
            rule.evaluation_periods = window;
            rule.datapoints_to_alarm = Some(to_alarm);

            assert!(
                settings.validate().is_err(),
                "{to_alarm} of {window} should be refused"
            );
        }
    }

    #[test]
    fn only_periods_cloudwatch_accepts_are_allowed() {
        for seconds in [1, 5, 10, 20, 30, 60, 300, 3600] {
            assert!(is_cloudwatch_period(seconds), "{seconds}");
        }
        for seconds in [0, 2, 45, 61, 359] {
            assert!(!is_cloudwatch_period(seconds), "{seconds}");
        }
    }

    /// The HCL spells these in camelCase and PascalCase respectively; a port that has to translate
    /// them is a port with a bug in it.
    #[test]
    fn the_catalogues_own_spellings_deserialize() {
        assert_eq!(
            serde_json::from_value::<TreatMissingData>(serde_json::json!("notBreaching")).unwrap(),
            TreatMissingData::NotBreaching
        );
        assert_eq!(
            serde_json::from_value::<Statistic>(serde_json::json!("Sum")).unwrap(),
            Statistic::Sum
        );
        assert_eq!(
            serde_json::from_value::<ComparisonOperator>(serde_json::json!(
                "LessThanOrEqualToThreshold"
            ))
            .unwrap(),
            ComparisonOperator::LessThanOrEqualToThreshold
        );
    }
}
