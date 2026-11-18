use common_types::alert_rules::{CloudwatchStatistic, GrafanaCloudwatchQuery};
use external_services::metrics_provider::grafana::cloudwatch::{
    CloudwatchMetricsQuery, CloudwatchStatistic as ProviderStatistic, GrafanaCloudwatch,
};
use time::PrimitiveDateTime;

use crate::{
    core::alert_evaluation::{evaluate_metrics, AlertEvaluator},
    domain_models::evaluator::{Evaluation, EvaluatorResult, RuleDefinition},
};

#[async_trait::async_trait]
impl AlertEvaluator<GrafanaCloudwatchQuery> for GrafanaCloudwatch {
    async fn evaluate(
        &self,
        rule: &RuleDefinition,
        query: &GrafanaCloudwatchQuery,
        datasource_uid: &str,
        now: PrimitiveDateTime,
    ) -> EvaluatorResult<Vec<Evaluation>> {
        evaluate_metrics(
            self,
            metrics_query(query, datasource_uid),
            query.period,
            query.end_offset_seconds,
            &rule.config.thresholds,
            now,
        )
        .await
        .map(|evaluation| vec![evaluation])
    }
}

pub fn metrics_query(
    query: &GrafanaCloudwatchQuery,
    datasource_uid: &str,
) -> CloudwatchMetricsQuery {
    CloudwatchMetricsQuery {
        datasource_uid: datasource_uid.to_owned(),
        region: query.region.clone(),
        namespace: query.namespace.clone(),
        metric_name: query.metric_name.clone(),
        dimensions: query.dimensions.clone(),
        statistic: match query.statistic {
            CloudwatchStatistic::Average => ProviderStatistic::Average,
            CloudwatchStatistic::Maximum => ProviderStatistic::Maximum,
            CloudwatchStatistic::Minimum => ProviderStatistic::Minimum,
            CloudwatchStatistic::Sum => ProviderStatistic::Sum,
        },
        period: query.period,
    }
}
