//! The infrastructure alarm catalogue, transcribed from the CloudWatch alarm rules the
//! infrastructure repository owns.
//!
//! Shaped for environment variables, since that is how a deployment sets it: ids are lowercase and
//! `_`-separated because `config` lowercases keys and an environment variable name cannot contain
//! `-`, and severities nest under their definition so metric and dimensions are set once.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use common_utils::ext_traits::ConfigExt;
use external_services::metrics_service::{Aggregation, Labels, Period};
use serde::{de, Deserialize, Deserializer};

use crate::{errors, settings::validate_config_ids};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CloudWatchSettings {
    pub alarms: HashMap<String, AlarmDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlarmDefinition {
    /// The name upstream, e.g. `rds-primary-cpu`. Separate from the id, which cannot carry `-`.
    pub name: String,
    pub classification: String,
    pub metric_name: String,
    pub namespace: String,
    #[serde(default)]
    pub dimensions: Dimensions,
    pub period: u32,
    pub statistic: Statistic,
    pub severities: HashMap<String, SeverityRule>,
}

/// An independent rule over the definition's readings. Severities do not suppress each other.
#[derive(Debug, Clone, Deserialize)]
pub struct SeverityRule {
    pub threshold: f64,
    pub comparison_operator: ComparisonOperator,
    pub evaluation_periods: u32,
    /// CloudWatch's M of N. Absent means all of them.
    #[serde(default)]
    pub datapoints_to_alarm: Option<u32>,
    pub treat_missing_data: MissingDataPolicy,
    /// The message delivered when this rule fires.
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Statistic {
    Average,
    Maximum,
    Minimum,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ComparisonOperator {
    GreaterThanThreshold,
    GreaterThanOrEqualToThreshold,
    LessThanThreshold,
    LessThanOrEqualToThreshold,
}

impl ComparisonOperator {
    pub fn breaches(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThanThreshold => value > threshold,
            Self::GreaterThanOrEqualToThreshold => value >= threshold,
            Self::LessThanThreshold => value < threshold,
            Self::LessThanOrEqualToThreshold => value <= threshold,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingDataPolicy {
    Breaching,
    NotBreaching,
    Ignore,
    Missing,
}

/// Written as `Name=value` pairs rather than a table because `config` lowercases keys read from
/// the environment, and CloudWatch answers a lowercased dimension name with no data rather than an
/// error. A table is accepted too, for files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dimensions(BTreeMap<String, String>);

impl Dimensions {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn labels(&self) -> Labels {
        self.iter().collect()
    }

    fn parse(text: &str) -> Result<Self, String> {
        let mut dimensions = Self::default();

        for pair in text.split(',').filter(|pair| !pair.trim().is_empty()) {
            let (name, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("`{}` is not a `Name=value` pair", pair.trim()))?;
            dimensions.insert(name.trim(), value.trim())?;
        }

        Ok(dimensions)
    }

    fn insert(&mut self, name: &str, value: &str) -> Result<(), String> {
        if name.is_empty() || value.is_empty() {
            Err(format!("`{name}={value}` has an empty name or value"))?
        }
        if self.0.insert(name.to_owned(), value.to_owned()).is_some() {
            Err(format!("`{name}` is set twice"))?
        }
        Ok(())
    }
}

impl fmt::Display for Dimensions {
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
                let mut dimensions = Dimensions::default();
                while let Some((name, value)) = map.next_entry::<String, String>()? {
                    dimensions
                        .insert(name.trim(), value.trim())
                        .map_err(de::Error::custom)?;
                }
                Ok(dimensions)
            }
        }

        deserializer.deserialize_any(DimensionsVisitor)
    }
}

impl CloudWatchSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_config_ids(&self.alarms, "cloudwatch alarm")?;

        for (id, alarm) in &self.alarms {
            alarm.validate(id)?;
        }

        Ok(())
    }

    pub fn rule_count(&self) -> usize {
        self.alarms
            .values()
            .map(|alarm| alarm.severities.len())
            .sum()
    }
}

impl AlarmDefinition {
    pub fn period(&self) -> Period {
        Period::from_seconds(i32::try_from(self.period).unwrap_or(i32::MAX))
    }

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

        // The provider rejects the whole batch over one unusable period, so an alarm configured
        // with 61 would blind every other metric evaluated alongside it.
        if !is_queryable_period(self.period) {
            return reject(format!(
                "period must be 1, 5, 10, 20, 30 or a multiple of 60 seconds, not {}",
                self.period
            ));
        }

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

/// The periods `metrics_service`'s CloudWatch backend accepts, capped at a day.
fn is_queryable_period(seconds: u32) -> bool {
    let high_resolution = matches!(seconds, 1 | 5 | 10 | 20 | 30);
    let standard = seconds > 0 && seconds % 60 == 0 && seconds <= 86_400;

    high_resolution || standard
}

impl SeverityRule {
    fn validate(&self, alarm: &str, severity: &str) -> Result<(), errors::ConfigurationError> {
        let reject = |reason: String| {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "cloudwatch alarm `{alarm}` severity `{severity}`: {reason}"
            )))
        };

        // NaN compares false against every reading, so the alarm could never fire.
        if !self.threshold.is_finite() {
            return reject(format!(
                "threshold must be a finite number, not {}",
                self.threshold
            ));
        }

        if self.evaluation_periods == 0 {
            return reject("evaluation_periods must be at least 1".to_owned());
        }

        if let Some(datapoints) = self.datapoints_to_alarm {
            if datapoints == 0 || datapoints > self.evaluation_periods {
                return reject(format!(
                    "datapoints_to_alarm must be between 1 and evaluation_periods ({}), not {datapoints}",
                    self.evaluation_periods
                ));
            }
        }

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

    /// Reads through the production path — same prefix, separator and parsing as
    /// [`Settings::with_config_path`] — but from a supplied map rather than the process
    /// environment.
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

    #[test]
    fn a_definition_is_read_from_the_environment() {
        let settings = settings_from(rds_primary_cpu_environment());
        let alarm = &settings.cloudwatch.alarms["rds_primary_cpu"];

        assert_eq!(alarm.name, "rds-primary-cpu");
        assert_eq!(alarm.classification, "rds-alerts");
        assert_eq!(alarm.metric_name, "CPUUtilization");
        assert_eq!(alarm.namespace, "AWS/RDS");
        assert_eq!(alarm.period(), Period::ONE_MINUTE);
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

    /// A lowercased `dbinstanceidentifier` selects nothing and reports no error, so this is the
    /// test that fails if the string form is ever replaced by a table.
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

    #[test]
    fn dimensions_can_also_be_written_as_a_table() {
        let dimensions: Dimensions =
            serde_json::from_str(r#"{"DBInstanceIdentifier":"hyperswitchdb-primary"}"#).unwrap();

        assert_eq!(
            dimensions.iter().collect::<Vec<_>>(),
            vec![("DBInstanceIdentifier", "hyperswitchdb-primary")]
        );
    }

    /// The table form takes the same checks as the string form; an empty value would otherwise
    /// deserialize into a selector CloudWatch cannot answer.
    #[test]
    fn a_dimension_table_with_an_empty_name_or_value_is_rejected() {
        for json in [
            r#"{"Role":""}"#,
            r#"{"":"hyperswitchdb-primary"}"#,
            r#"{"Role":"  "}"#,
        ] {
            assert!(
                serde_json::from_str::<Dimensions>(json).is_err(),
                "`{json}` should be rejected"
            );
        }
    }

    fn catalogue_with(id: &str, alarm: AlarmDefinition) -> CloudWatchSettings {
        CloudWatchSettings {
            alarms: [(id.to_owned(), alarm)].into_iter().collect(),
        }
    }

    fn rds_primary_cpu() -> AlarmDefinition {
        settings_from(rds_primary_cpu_environment())
            .cloudwatch
            .alarms["rds_primary_cpu"]
            .clone()
    }

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
            ("a period the provider would reject the whole batch over", {
                let mut alarm = rds_primary_cpu();
                alarm.period = 61;
                catalogue_with("rds_primary_cpu", alarm)
            }),
            ("a period beyond a day", {
                let mut alarm = rds_primary_cpu();
                alarm.period = 86_460;
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

    #[test]
    fn the_periods_the_catalogue_uses_are_accepted() {
        for seconds in [1, 5, 10, 20, 30, 60, 300, 3600, 86_400] {
            assert!(is_queryable_period(seconds), "{seconds} should be usable");
        }
        for seconds in [0, 2, 61, 90, 86_460] {
            assert!(!is_queryable_period(seconds), "{seconds} should not be");
        }
    }

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
