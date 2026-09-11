//! The infrastructure alert catalogue, transcribed from the CloudWatch alarms the infrastructure
//! repository owns.
//!
//! A [`Definition`] says what to measure; the [`Rule`]s under it each say what is bad about it.
//! One rule corresponds to one of the AWS alarms we run alongside, which is why "alarm" is not
//! used for anything here.
//!
//! Shaped for environment variables, since that is how a deployment sets it: ids are lowercase and
//! `_`-separated because `config` lowercases keys and an environment variable name cannot contain
//! `-`, and severities nest under their definition so metric and dimensions are set once.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use common_utils::ext_traits::ConfigExt;
use external_services::metrics_service::{
    aws_cloudwatch::CloudWatchConfig, Aggregation, Labels, Period,
};
use serde::Deserialize;

use crate::{
    errors,
    settings::utils::{deserialize_hashset, validate_config_ids},
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CloudWatchSettings {
    /// Where to read the metrics from. Empty means no provider is built, which is only valid
    /// while there is nothing to evaluate.
    pub client: CloudWatchConfig,
    /// Which chat destination each severity announces to. Every severity the catalogue
    /// uses must appear, or an alarm would be evaluated and then silently dropped.
    pub destinations: HashMap<String, String>,
    pub definitions: HashMap<String, Definition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Definition {
    /// The name upstream, e.g. `rds-primary-cpu`. Separate from the id, which cannot carry `-`.
    pub name: String,
    pub classification: String,
    pub metric_name: String,
    pub namespace: String,
    /// `Name=value` pairs separated by commas.
    #[serde(default, deserialize_with = "deserialize_hashset")]
    pub dimensions: HashSet<Dimension>,
    pub period: u32,
    pub statistic: Statistic,
    pub severities: HashMap<String, Rule>,
}

/// An independent rule over the definition's readings. Severities do not suppress each other.
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
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

/// One dimension, written `Name=value`.
///
/// A pair rather than a table entry because `config` lowercases keys read from the environment,
/// and CloudWatch answers a lowercased dimension name with no data rather than an error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub value: String,
}

impl FromStr for Dimension {
    type Err = String;

    fn from_str(pair: &str) -> Result<Self, Self::Err> {
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("`{}` is not a `Name=value` pair", pair.trim()))?;
        let (name, value) = (name.trim(), value.trim());

        if name.is_empty() || value.is_empty() {
            Err(format!("`{}` has an empty name or value", pair.trim()))?
        }

        Ok(Self {
            name: name.to_owned(),
            value: value.to_owned(),
        })
    }
}

impl CloudWatchSettings {
    pub fn validate(&self) -> Result<(), errors::ConfigurationError> {
        validate_config_ids(&self.definitions, "cloudwatch definition")?;

        for (id, definition) in &self.definitions {
            definition.validate(id)?;
        }

        // A catalogue with nowhere to read from evaluates to nothing at all, which looks from the
        // outside like a service reporting that everything is fine.
        if !self.definitions.is_empty() {
            self.client.validate().map_err(|reason| {
                errors::ConfigurationError::ConfigParsingError(format!(
                    "{reason}, because {} cloudwatch definitions are configured",
                    self.definitions.len()
                ))
            })?;
        }

        // An alarm with nowhere to announce to is worse than one that never fires: it is evaluated,
        // found breaching, and dropped.
        for (id, definition) in &self.definitions {
            for severity in definition.severities.keys() {
                if !self.destinations.contains_key(severity) {
                    Err(errors::ConfigurationError::ConfigParsingError(format!(
                        "cloudwatch definition `{id}` announces at severity `{severity}`, which \
                         has no entry in cloudwatch.destinations"
                    )))?
                }
            }
        }

        Ok(())
    }

    /// The severities that would be announced, and where each of them goes.
    pub fn destination_ids(&self) -> impl Iterator<Item = (&str, &str)> {
        self.destinations
            .iter()
            .map(|(severity, destination)| (severity.as_str(), destination.as_str()))
    }

    pub fn rule_count(&self) -> usize {
        self.definitions
            .values()
            .map(|definition| definition.severities.len())
            .sum()
    }
}

impl Definition {
    pub fn labels(&self) -> Labels {
        self.dimensions
            .iter()
            .map(|dimension| (dimension.name.as_str(), dimension.value.as_str()))
            .collect()
    }

    pub fn period(&self) -> Period {
        Period::from_seconds(i32::try_from(self.period).unwrap_or(i32::MAX))
    }

    fn validate(&self, id: &str) -> Result<(), errors::ConfigurationError> {
        let reject = |reason: String| {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "cloudwatch definition `{id}`: {reason}"
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

        // The provider rejects the whole batch over one unusable period, so a definition configured
        // with 61 would blind every other metric evaluated alongside it.
        if !is_queryable_period(self.period) {
            return reject(format!(
                "period must be 1, 5, 10, 20, 30 or a multiple of 60 seconds, not {}",
                self.period
            ));
        }

        let mut names = HashSet::with_capacity(self.dimensions.len());
        for dimension in &self.dimensions {
            if !names.insert(dimension.name.as_str()) {
                return reject(format!("dimension `{}` is set twice", dimension.name));
            }
        }

        if self.severities.is_empty() {
            return reject("must define at least one severity".to_owned());
        }

        validate_config_ids(
            &self.severities,
            &format!("cloudwatch definition `{id}` severity"),
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

impl Rule {
    fn validate(&self, definition: &str, severity: &str) -> Result<(), errors::ConfigurationError> {
        let reject = |reason: String| {
            Err(errors::ConfigurationError::ConfigParsingError(format!(
                "cloudwatch definition `{definition}` severity `{severity}`: {reason}"
            )))
        };

        // NaN compares false against every reading, so the rule could never fire.
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
    use crate::settings::{utils::deserialize_hashset_inner, Settings};

    fn rds_primary_cpu_environment() -> HashMap<String, String> {
        [
            ("OBSERVABILITY__CLOUDWATCH__CLIENT__REGION", "ap-south-1"),
            ("OBSERVABILITY__CLOUDWATCH__DESTINATIONS__SEV1", "infra_alerts"),
            ("OBSERVABILITY__CLOUDWATCH__DESTINATIONS__SEV3", "infra_alerts"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__NAME", "rds-primary-cpu"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__CLASSIFICATION", "rds-alerts"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__METRIC_NAME", "CPUUtilization"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__NAMESPACE", "AWS/RDS"),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__DIMENSIONS",
                "DBInstanceIdentifier=hyperswitchdb-primary",
            ),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__PERIOD", "60"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__STATISTIC", "Average"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__THRESHOLD", "90"),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__COMPARISON_OPERATOR",
                "GreaterThanOrEqualToThreshold",
            ),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__EVALUATION_PERIODS", "1"),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__TREAT_MISSING_DATA",
                "breaching",
            ),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV1__DESCRIPTION",
                "SEV1: RDS primary database CPU utilization is above 90%.",
            ),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__THRESHOLD", "65"),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__COMPARISON_OPERATOR",
                "GreaterThanOrEqualToThreshold",
            ),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__EVALUATION_PERIODS", "3"),
            ("OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__DATAPOINTS_TO_ALARM", "2"),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__TREAT_MISSING_DATA",
                "notBreaching",
            ),
            (
                "OBSERVABILITY__CLOUDWATCH__DEFINITIONS__RDS_PRIMARY_CPU__SEVERITIES__SEV3__DESCRIPTION",
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
        let definition = &settings.cloudwatch.definitions["rds_primary_cpu"];

        assert_eq!(definition.name, "rds-primary-cpu");
        assert_eq!(definition.classification, "rds-alerts");
        assert_eq!(definition.metric_name, "CPUUtilization");
        assert_eq!(definition.namespace, "AWS/RDS");
        assert_eq!(definition.period(), Period::ONE_MINUTE);
        assert_eq!(
            Aggregation::from(definition.statistic),
            Aggregation::Average
        );

        let sev1 = &definition.severities["sev1"];
        assert!((sev1.threshold - 90.0).abs() < f64::EPSILON);
        assert_eq!(
            sev1.comparison_operator,
            ComparisonOperator::GreaterThanOrEqualToThreshold
        );
        assert_eq!(sev1.evaluation_periods, 1);
        assert_eq!(sev1.datapoints_to_alarm, None);
        assert_eq!(sev1.treat_missing_data, MissingDataPolicy::Breaching);

        let sev3 = &definition.severities["sev3"];
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
        let definition = &settings.cloudwatch.definitions["rds_primary_cpu"];

        assert_eq!(
            definition.dimensions,
            [dimension("DBInstanceIdentifier", "hyperswitchdb-primary")]
                .into_iter()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            definition.labels(),
            [("DBInstanceIdentifier", "hyperswitchdb-primary")]
                .into_iter()
                .collect::<Labels>()
        );
    }

    fn dimension(name: &str, value: &str) -> Dimension {
        Dimension {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn several_dimensions_are_comma_separated_and_may_be_spaced() {
        let dimensions: HashSet<Dimension> =
            deserialize_hashset_inner(" DBClusterIdentifier=hyperswitchdb-cluster , Role=WRITER ")
                .unwrap();

        assert_eq!(
            dimensions,
            [
                dimension("DBClusterIdentifier", "hyperswitchdb-cluster"),
                dimension("Role", "WRITER"),
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        );
    }

    /// Only the first `=` separates, so a value containing one survives.
    #[test]
    fn a_value_may_contain_the_separator() {
        assert_eq!(
            "LoadBalancer=app/envoy-alb/123=456"
                .parse::<Dimension>()
                .unwrap(),
            dimension("LoadBalancer", "app/envoy-alb/123=456")
        );
    }

    #[test]
    fn a_dimension_that_is_not_a_pair_is_rejected() {
        for text in ["DBInstanceIdentifier", "=value", "Name=", "Name=  ", ""] {
            assert!(
                text.parse::<Dimension>().is_err(),
                "`{text}` should be rejected"
            );
        }
    }

    /// Carries a usable client, so a case below is rejected for the reason it names rather than
    /// for having nowhere to read from.
    fn catalogue_with(id: &str, definition: Definition) -> CloudWatchSettings {
        CloudWatchSettings {
            destinations: [("sev1", "infra"), ("sev3", "infra")]
                .into_iter()
                .map(|(severity, id)| (severity.to_owned(), id.to_owned()))
                .collect(),
            client: CloudWatchConfig {
                region: "ap-south-1".to_owned(),
            },
            definitions: [(id.to_owned(), definition)].into_iter().collect(),
        }
    }

    fn rds_primary_cpu() -> Definition {
        settings_from(rds_primary_cpu_environment())
            .cloudwatch
            .definitions["rds_primary_cpu"]
            .clone()
    }

    #[test]
    fn a_catalogue_that_cannot_be_evaluated_faithfully_is_rejected() {
        let cases: Vec<(&str, CloudWatchSettings)> = vec![
            ("an id that cannot be set from the environment", {
                catalogue_with("RDS_PRIMARY_CPU", rds_primary_cpu())
            }),
            ("no severities at all", {
                let mut definition = rds_primary_cpu();
                definition.severities.clear();
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("a zero period", {
                let mut definition = rds_primary_cpu();
                definition.period = 0;
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("a period the provider would reject the whole batch over", {
                let mut definition = rds_primary_cpu();
                definition.period = 61;
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("a period beyond a day", {
                let mut definition = rds_primary_cpu();
                definition.period = 86_460;
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("the same dimension twice", {
                let mut definition = rds_primary_cpu();
                definition.dimensions = [
                    dimension("DBClusterIdentifier", "hyperswitchdb-cluster"),
                    dimension("DBClusterIdentifier", "failover-replica-1"),
                ]
                .into_iter()
                .collect();
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("an empty metric name", {
                let mut definition = rds_primary_cpu();
                definition.metric_name = String::new();
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("a window that can never fill", {
                let mut definition = rds_primary_cpu();
                if let Some(rule) = definition.severities.get_mut("sev3") {
                    rule.datapoints_to_alarm = Some(rule.evaluation_periods + 1);
                }
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("a window of no periods", {
                let mut definition = rds_primary_cpu();
                if let Some(rule) = definition.severities.get_mut("sev1") {
                    rule.evaluation_periods = 0;
                }
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("an alert with nothing to say", {
                let mut definition = rds_primary_cpu();
                if let Some(rule) = definition.severities.get_mut("sev1") {
                    rule.description = String::new();
                }
                catalogue_with("rds_primary_cpu", definition)
            }),
            ("a threshold no reading can be compared against", {
                let mut definition = rds_primary_cpu();
                if let Some(rule) = definition.severities.get_mut("sev1") {
                    rule.threshold = f64::NAN;
                }
                catalogue_with("rds_primary_cpu", definition)
            }),
            // Nowhere to read from evaluates to nothing at all, which from outside is
            // indistinguishable from a service reporting that everything is fine.
            // Evaluated, found breaching, and then dropped — the one failure this service
            // cannot report on its own.
            ("a severity with nowhere to announce to", {
                let mut catalogue = catalogue_with("rds_primary_cpu", rds_primary_cpu());
                catalogue.destinations.remove("sev3");
                catalogue
            }),
            ("definitions but no region to read them from", {
                let mut catalogue = catalogue_with("rds_primary_cpu", rds_primary_cpu());
                catalogue.client = CloudWatchConfig::default();
                catalogue
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
    fn an_empty_catalogue_is_accepted_without_a_region() {
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
