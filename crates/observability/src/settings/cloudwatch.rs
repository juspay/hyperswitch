//! The infrastructure alarm catalogue this service evaluates.
//!
//! These definitions are not authored here. They are the CloudWatch alarm rules the infrastructure
//! repository already owns — the same rules that create the AWS alarms running today — carried
//! into this process so it can evaluate them itself. Thresholds are transcribed, never invented:
//! an alarm that fires here and an alarm that fires in CloudWatch should be the same alarm.
//!
//! # Why this is shaped for environment variables
//!
//! The deployment renders configuration into `OBSERVABILITY__`-prefixed environment variables, and
//! that mechanism decides the shape more than taste does:
//!
//! - **Ids are map keys, so they arrive lowercased and `_`-separated.** `config` lowercases every
//!   environment key before splitting it on `__`, and an environment variable name cannot contain
//!   `-` at all. So the catalogue's `rds-primary-cpu` is keyed `rds_primary_cpu` here, and
//!   [`AlarmDefinition::name`] carries the name it has upstream rather than leaving the two to be
//!   reconciled by guesswork.
//! - **Dimensions are one string, not a nested table.** CloudWatch dimension names are
//!   case-sensitive (`DBInstanceIdentifier`), and a nested table would lose that casing to the same
//!   lowercasing — silently selecting no metric stream at all rather than failing. As a single
//!   value the casing survives, because `config` lowercases keys and leaves values alone.
//! - **Severities nest under their definition.** Metric, namespace and dimensions are properties of
//!   the thing being watched, not of how alarming it is, so they are set once. Two severities of
//!   one definition cannot end up watching different metrics.
//!
//! # What is deliberately absent
//!
//! No defaults for the rule fields. The upstream catalogue leaves `period`, `statistic`,
//! `comparison_operator`, `evaluation_periods` and `treat_missing_data` unset on most entries and
//! its Terraform module fills them in; repeating those defaults here would be a second copy of
//! somebody else's decision, free to drift from it. Whatever renders the configuration resolves
//! them, and this service refuses a rule that does not say what it evaluates. `datapoints_to_alarm`
//! is the exception: absent genuinely means "all of them" in CloudWatch, so it is an [`Option`].

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use common_utils::ext_traits::ConfigExt;
use external_services::metrics_service::{Aggregation, Labels, Period};
use serde::{de, Deserialize, Deserializer};

use crate::{errors, settings::validate_config_ids};

/// The largest period CloudWatch accepts for an alarm, one day, in seconds.
const MAX_PERIOD_SECONDS: u32 = 86_400;

/// The alarm catalogue, and anything else this service needs to read CloudWatch.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CloudWatchSettings {
    /// Every alarm definition, by id.
    ///
    /// Empty is a valid configuration, not a misconfiguration: a deployment that has not been
    /// given a catalogue yet still serves every other route.
    pub alarms: HashMap<String, AlarmDefinition>,
}

/// One metric being watched, and the severities watching it.
#[derive(Debug, Clone, Deserialize)]
pub struct AlarmDefinition {
    /// The name this definition has in the source catalogue, e.g. `rds-primary-cpu`.
    ///
    /// Its id here cannot be that name — environment variable names have no `-` — so the real name
    /// travels as a value. It is what an operator greps for when an alert they received has to be
    /// traced back to the rule that produced it.
    pub name: String,

    /// The group the definition belongs to upstream, e.g. `rds-alerts`. Carried because alarms are
    /// ported a classification at a time, so "which of these came from the cache port?" is a
    /// question worth being able to answer.
    pub classification: String,

    /// The metric to read, e.g. `CPUUtilization`.
    pub metric_name: String,

    /// The namespace the metric is published under, e.g. `AWS/RDS`.
    pub namespace: String,

    /// The dimensions selecting one reporting stream, as `Name=value` pairs separated by commas.
    ///
    /// Already resolved to concrete values — no group references, no templates. Whatever renders
    /// this configuration knows which instances exist; this service only reads what it is given.
    #[serde(default)]
    pub dimensions: Dimensions,

    /// How many seconds one datapoint covers.
    pub period: u32,

    /// How the values inside a period are reduced to the number compared against the threshold.
    pub statistic: Statistic,

    /// The severity rules, by id — `sev1`, `sev2`, `sev3`.
    ///
    /// Each is an independent rule over the same readings. A more severe rule does not suppress a
    /// less severe one; both announce, and both say which they are.
    pub severities: HashMap<String, SeverityRule>,
}

/// One severity's rule: the threshold, and how long it has to hold.
#[derive(Debug, Clone, Deserialize)]
pub struct SeverityRule {
    /// The value the metric is compared against.
    pub threshold: f64,

    /// Which side of the threshold is a breach.
    pub comparison_operator: ComparisonOperator,

    /// How many periods the evaluation window covers.
    pub evaluation_periods: u32,

    /// How many of those periods must breach for the alarm to fire — CloudWatch's "M out of N".
    ///
    /// Absent means all of them, which is CloudWatch's own default rather than a convention
    /// invented here.
    #[serde(default)]
    pub datapoints_to_alarm: Option<u32>,

    /// What a period with no reading counts as.
    pub treat_missing_data: MissingDataPolicy,

    /// The operator-facing sentence this rule exists to send.
    ///
    /// Hand-written upstream, addressed to whoever is woken by it, and the reason severities are
    /// separate rules rather than a list of thresholds: each carries its own instruction.
    pub description: String,
}

/// How the values inside one period are reduced.
///
/// Spelled as CloudWatch spells them, because the configuration is transcribed from CloudWatch
/// alarm definitions and a rename would only make the two harder to diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Statistic {
    /// Arithmetic mean of the values in the period.
    Average,
    /// Largest value in the period.
    Maximum,
    /// Smallest value in the period.
    Minimum,
    /// Every value in the period added together.
    Sum,
}

impl From<Statistic> for Aggregation {
    fn from(statistic: Statistic) -> Self {
        match statistic {
            Statistic::Average => Self::Average,
            Statistic::Maximum => Self::Maximum,
            Statistic::Minimum => Self::Minimum,
            Statistic::Sum => Self::Sum,
        }
    }
}

/// Which side of the threshold counts as breaching.
///
/// The four the catalogue uses. CloudWatch's anomaly-detection operators are absent on purpose:
/// they compare against a band rather than a number, and a definition carrying one has no
/// `threshold` to put in this configuration at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ComparisonOperator {
    /// Breaching above the threshold.
    GreaterThanThreshold,
    /// Breaching at or above the threshold.
    GreaterThanOrEqualToThreshold,
    /// Breaching below the threshold.
    LessThanThreshold,
    /// Breaching at or below the threshold.
    LessThanOrEqualToThreshold,
}

impl ComparisonOperator {
    /// Whether `value` breaches `threshold` under this operator.
    pub fn breaches(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThanThreshold => value > threshold,
            Self::GreaterThanOrEqualToThreshold => value >= threshold,
            Self::LessThanThreshold => value < threshold,
            Self::LessThanOrEqualToThreshold => value <= threshold,
        }
    }
}

/// What a period with no reading counts as.
///
/// CloudWatch's four policies, spelled as the catalogue spells them. The distinction matters more
/// than it looks: on `Breaching`, a metric that stops reporting *is* the alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingDataPolicy {
    /// A missing period counts as a breach.
    Breaching,
    /// A missing period counts as healthy.
    NotBreaching,
    /// A missing period is skipped, and the window keeps looking further back.
    Ignore,
    /// A missing period leaves the alarm without enough data to say.
    Missing,
}

/// The dimensions selecting one metric stream.
///
/// Ordered, so the same set built two ways selects identically.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dimensions(BTreeMap<String, String>);

impl Dimensions {
    /// The dimensions, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// Whether any dimension is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The same selectors, as the metrics client's provider-neutral labels.
    pub fn labels(&self) -> Labels {
        self.iter().collect()
    }

    /// Parse the `Name=value,Other=value` form the environment carries.
    fn parse(text: &str) -> Result<Self, String> {
        let mut dimensions = BTreeMap::new();

        for pair in text.split(',').filter(|pair| !pair.trim().is_empty()) {
            let (name, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("`{}` is not a `Name=value` pair", pair.trim()))?;
            let (name, value) = (name.trim(), value.trim());

            if name.is_empty() || value.is_empty() {
                Err(format!("`{}` has an empty name or value", pair.trim()))?
            }
            if dimensions
                .insert(name.to_owned(), value.to_owned())
                .is_some()
            {
                Err(format!("`{name}` is set twice"))?
            }
        }

        Ok(Self(dimensions))
    }
}

impl fmt::Display for Dimensions {
    /// The same form the configuration is written in, so a log line can be pasted back.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (name, value) in self.iter() {
            if !first {
                write!(formatter, ",")?;
            }
            write!(formatter, "{name}={value}")?;
            first = false;
        }
        Ok(())
    }
}

/// Accepts both forms this can arrive in: the flat string an environment variable carries, and the
/// table a file can express. The string form is the one that matters — it is the only one that
/// preserves CloudWatch's case-sensitive dimension names through `config`'s lowercasing of keys.
impl<'de> Deserialize<'de> for Dimensions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DimensionsVisitor;

        impl<'de> de::Visitor<'de> for DimensionsVisitor {
            type Value = Dimensions;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("`Name=value` pairs separated by commas, or a table of them")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
                Dimensions::parse(text).map_err(de::Error::custom)
            }

            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut dimensions = BTreeMap::new();
                while let Some((name, value)) = map.next_entry::<String, String>()? {
                    dimensions.insert(name, value);
                }
                Ok(Dimensions(dimensions))
            }
        }

        deserializer.deserialize_any(DimensionsVisitor)
    }
}

impl CloudWatchSettings {
    /// Reject a catalogue this service cannot evaluate faithfully.
    ///
    /// Every check here is something that would otherwise become a wrong answer rather than an
    /// error: an alarm that silently watches nothing, a window that can never fill, a threshold
    /// that no reading can be compared against.
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_config_ids(&self.alarms, "cloudwatch alarm")?;

        for (id, alarm) in &self.alarms {
            alarm.validate(id)?;
        }

        Ok(())
    }

    /// How many severity rules the catalogue holds, across every definition.
    ///
    /// The count worth logging at boot: definitions are what an operator edits, but rules are what
    /// this service evaluates and delivers.
    pub fn rule_count(&self) -> usize {
        self.alarms
            .values()
            .map(|alarm| alarm.severities.len())
            .sum()
    }
}

impl AlarmDefinition {
    /// The period, as the metrics client expresses it.
    ///
    /// The cast cannot saturate: [`AlarmDefinition::validate`] rejects anything above
    /// [`MAX_PERIOD_SECONDS`], which is far below [`i32::MAX`].
    pub fn period(&self) -> Period {
        Period::from_seconds(i32::try_from(self.period).unwrap_or(i32::MAX))
    }

    /// Reject a definition that cannot be evaluated as written.
    fn validate(&self, id: &str) -> Result<(), errors::ConfigurationError> {
        let reject = |reason: String| {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "cloudwatch alarm `{id}`: {reason}"
            )))
        };

        for (field, value) in [
            ("name", &self.name),
            ("classification", &self.classification),
            ("metric_name", &self.metric_name),
            ("namespace", &self.namespace),
        ] {
            if value.is_default_or_empty() {
                return reject(format!("{field} must not be empty"));
            }
        }

        // A period of zero divides the window into nothing; one above CloudWatch's maximum is a
        // request the provider will reject at query time, which is a worse place to find out.
        if self.period == 0 || self.period > MAX_PERIOD_SECONDS {
            return reject(format!(
                "period must be between 1 and {MAX_PERIOD_SECONDS} seconds, not {}",
                self.period
            ));
        }

        // An empty severity map is an alarm that watches a metric and can never say anything about
        // it — configuration that looks complete and does nothing.
        if self.severities.is_empty() {
            return reject("must define at least one severity".to_owned());
        }

        validate_config_ids(
            &self.severities,
            &format!("cloudwatch alarm `{id}` severity"),
        )?;

        for (severity, rule) in &self.severities {
            rule.validate(id, severity)?;
        }

        Ok(())
    }
}

impl SeverityRule {
    /// Reject a rule whose window or threshold cannot decide anything.
    fn validate(&self, alarm: &str, severity: &str) -> Result<(), errors::ConfigurationError> {
        let reject = |reason: String| {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "cloudwatch alarm `{alarm}` severity `{severity}`: {reason}"
            )))
        };

        // NaN compares false against everything, so a NaN threshold is an alarm that can never
        // fire — indistinguishable at runtime from one that is simply never breached.
        if !self.threshold.is_finite() {
            return reject(format!(
                "threshold must be a finite number, not {}",
                self.threshold
            ));
        }

        if self.evaluation_periods == 0 {
            return reject("evaluation_periods must be at least 1".to_owned());
        }

        // M out of N with M greater than N can never be satisfied. CloudWatch rejects it too; this
        // catches it at boot rather than on the first evaluation.
        if let Some(datapoints) = self.datapoints_to_alarm {
            if datapoints == 0 || datapoints > self.evaluation_periods {
                return reject(format!(
                    "datapoints_to_alarm must be between 1 and evaluation_periods ({}), not {datapoints}",
                    self.evaluation_periods
                ));
            }
        }

        // The description is not documentation, it is the message that gets delivered. An empty
        // one means an alert arrives saying nothing.
        if self.description.is_default_or_empty() {
            return reject("description must not be empty".to_owned());
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret;

    use super::*;
    use crate::settings::Settings;

    /// The environment a deployment would hand this service for one RDS definition, spelled the way
    /// the rendered configuration spells it.
    fn rds_primary_cpu_environment() -> HashMap<String, String> {
        [
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__NAME", "rds-primary-cpu"),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__CLASSIFICATION", "rds-alerts"),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__METRIC_NAME", "CPUUtilization"),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__NAMESPACE", "AWS/RDS"),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__DIMENSIONS",
                "DBInstanceIdentifier=hyperswitchdb-primary",
            ),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__PERIOD", "60"),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__STATISTIC", "Average"),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__THRESHOLD", "90"),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__COMPARISON_OPERATOR",
                "GreaterThanOrEqualToThreshold",
            ),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__EVALUATION_PERIODS", "1"),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__TREAT_MISSING_DATA",
                "breaching",
            ),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__DESCRIPTION",
                "SEV1: RDS primary database CPU utilization is above 90%.",
            ),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__THRESHOLD", "65"),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__COMPARISON_OPERATOR",
                "GreaterThanOrEqualToThreshold",
            ),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__EVALUATION_PERIODS", "3"),
            ("OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__DATAPOINTS_TO_ALARM", "2"),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__TREAT_MISSING_DATA",
                "notBreaching",
            ),
            (
                "OBSERVABILITY__CLOUDWATCH__ALARMS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__DESCRIPTION",
                "SEV3: RDS primary database CPU utilization is above 65%.",
            ),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
    }

    /// Deserialize a whole configuration from an environment, exactly as
    /// [`Settings::with_config_path`] does — same prefix, same separator, same parsing — but from a
    /// supplied map rather than the process environment, so the test says nothing about whichever
    /// other test is running beside it.
    fn settings_from(environment: HashMap<String, String>) -> Settings<SecuredSecret> {
        config::Config::builder()
            .add_source(
                config::Environment::with_prefix("OBSERVABILITY")
                    .try_parsing(true)
                    .separator("__")
                    .source(Some(environment)),
            )
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }

    /// The whole point of the exercise: a catalogue set entirely from the environment arrives as
    /// typed rules, with its numbers as numbers and its policies as policies.
    #[test]
    fn a_definition_is_read_from_the_environment() {
        let settings = settings_from(rds_primary_cpu_environment());
        let alarm = &settings.cloudwatch.alarms["rds_primary_cpu"];

        assert_eq!(alarm.name, "rds-primary-cpu");
        assert_eq!(alarm.classification, "rds-alerts");
        assert_eq!(alarm.metric_name, "CPUUtilization");
        assert_eq!(alarm.namespace, "AWS/RDS");
        assert_eq!(alarm.period, 60);
        assert_eq!(alarm.period(), Period::ONE_MINUTE);
        assert_eq!(alarm.statistic, Statistic::Average);
        assert_eq!(Aggregation::from(alarm.statistic), Aggregation::Average);

        let sev1 = &alarm.severities["sev1"];
        assert!((sev1.threshold - 90.0).abs() < f64::EPSILON);
        assert_eq!(
            sev1.comparison_operator,
            ComparisonOperator::GreaterThanOrEqualToThreshold
        );
        assert_eq!(sev1.evaluation_periods, 1);
        assert_eq!(sev1.datapoints_to_alarm, None);
        assert_eq!(sev1.treat_missing_data, MissingDataPolicy::Breaching);

        let sev3 = &alarm.severities["sev3"];
        assert_eq!(sev3.datapoints_to_alarm, Some(2));
        assert_eq!(sev3.treat_missing_data, MissingDataPolicy::NotBreaching);

        assert_eq!(settings.cloudwatch.rule_count(), 2);
        settings.cloudwatch.validate().unwrap();
    }

    /// The reason dimensions are one string rather than a nested table. `config` lowercases every
    /// key it takes from the environment, and CloudWatch would answer a lowercased
    /// `dbinstanceidentifier` with no data at all rather than with an error.
    #[test]
    fn dimension_names_survive_the_environment_with_their_case() {
        let settings = settings_from(rds_primary_cpu_environment());
        let dimensions = &settings.cloudwatch.alarms["rds_primary_cpu"].dimensions;

        assert_eq!(
            dimensions.iter().collect::<Vec<_>>(),
            vec![("DBInstanceIdentifier", "hyperswitchdb-primary")]
        );
        assert_eq!(
            dimensions.labels(),
            [("DBInstanceIdentifier", "hyperswitchdb-primary")]
                .into_iter()
                .collect::<Labels>()
        );
    }

    #[test]
    fn several_dimensions_are_comma_separated_and_may_be_spaced() {
        let dimensions =
            Dimensions::parse(" DBClusterIdentifier=hyperswitchdb-cluster , Role=WRITER ").unwrap();

        assert_eq!(
            dimensions.iter().collect::<Vec<_>>(),
            vec![
                ("DBClusterIdentifier", "hyperswitchdb-cluster"),
                ("Role", "WRITER"),
            ]
        );
        // Round-trips, so a logged selector can be pasted back into the configuration.
        assert_eq!(
            dimensions.to_string(),
            "DBClusterIdentifier=hyperswitchdb-cluster,Role=WRITER"
        );
    }

    #[test]
    fn a_dimension_that_is_not_a_pair_is_rejected() {
        for text in ["DBInstanceIdentifier", "=value", "Name=", "A=1,A=2"] {
            assert!(
                Dimensions::parse(text).is_err(),
                "`{text}` should be rejected"
            );
        }
    }

    /// A table is what a file can express; the environment cannot, which is why it is not the only
    /// accepted form.
    #[test]
    fn dimensions_can_also_be_written_as_a_table() {
        let dimensions: Dimensions =
            serde_json::from_str(r#"{"DBInstanceIdentifier":"hyperswitchdb-primary"}"#).unwrap();

        assert_eq!(
            dimensions.iter().collect::<Vec<_>>(),
            vec![("DBInstanceIdentifier", "hyperswitchdb-primary")]
        );
    }

    fn catalogue_with(id: &str, alarm: AlarmDefinition) -> CloudWatchSettings {
        CloudWatchSettings {
            alarms: [(id.to_owned(), alarm)].into_iter().collect(),
        }
    }

    fn rds_primary_cpu() -> AlarmDefinition {
        let settings = settings_from(rds_primary_cpu_environment());
        settings.cloudwatch.alarms["rds_primary_cpu"].clone()
    }

    /// Each of these would otherwise boot into a service that evaluates something other than what
    /// the catalogue says, or nothing at all.
    #[test]
    fn a_catalogue_that_cannot_be_evaluated_faithfully_is_rejected() {
        let cases: Vec<(&str, CloudWatchSettings)> = vec![
            ("an id that cannot be set from the environment", {
                catalogue_with("RDS_PRIMARY_CPU", rds_primary_cpu())
            }),
            ("no severities at all", {
                let mut alarm = rds_primary_cpu();
                alarm.severities.clear();
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("a zero period", {
                let mut alarm = rds_primary_cpu();
                alarm.period = 0;
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("a period beyond what CloudWatch accepts", {
                let mut alarm = rds_primary_cpu();
                alarm.period = MAX_PERIOD_SECONDS + 60;
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("an empty metric name", {
                let mut alarm = rds_primary_cpu();
                alarm.metric_name = String::new();
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("a window that can never fill", {
                let mut alarm = rds_primary_cpu();
                if let Some(rule) = alarm.severities.get_mut("sev3") {
                    rule.datapoints_to_alarm = Some(rule.evaluation_periods + 1);
                }
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("a window of no periods", {
                let mut alarm = rds_primary_cpu();
                if let Some(rule) = alarm.severities.get_mut("sev1") {
                    rule.evaluation_periods = 0;
                }
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("an alert with nothing to say", {
                let mut alarm = rds_primary_cpu();
                if let Some(rule) = alarm.severities.get_mut("sev1") {
                    rule.description = String::new();
                }
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("a threshold no reading can be compared against", {
                let mut alarm = rds_primary_cpu();
                if let Some(rule) = alarm.severities.get_mut("sev1") {
                    rule.threshold = f64::NAN;
                }
                catalogue_with("rds_primary_cpu", alarm)
            }),
        ];

        for (reason, catalogue) in cases {
            assert!(
                catalogue.validate().is_err(),
                "a catalogue with {reason} should be rejected"
            );
        }
    }

    /// An empty catalogue is a deployment that has not been given one yet, not a broken one.
    #[test]
    fn an_empty_catalogue_is_accepted() {
        CloudWatchSettings::default().validate().unwrap();
    }

    #[test]
    fn comparison_operators_agree_with_their_names() {
        assert!(ComparisonOperator::GreaterThanThreshold.breaches(91.0, 90.0));
        assert!(!ComparisonOperator::GreaterThanThreshold.breaches(90.0, 90.0));
        assert!(ComparisonOperator::GreaterThanOrEqualToThreshold.breaches(90.0, 90.0));
        assert!(ComparisonOperator::LessThanThreshold.breaches(89.0, 90.0));
        assert!(!ComparisonOperator::LessThanThreshold.breaches(90.0, 90.0));
        assert!(ComparisonOperator::LessThanOrEqualToThreshold.breaches(90.0, 90.0));
    }
}
