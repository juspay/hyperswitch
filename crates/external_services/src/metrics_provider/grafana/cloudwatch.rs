use std::collections::BTreeMap;

use serde::Serialize;

use super::{GrafanaClient, QUERY_REF_ID};
use crate::metrics_provider::{MetricRange, MetricSeries, MetricsProvider, MetricsResult};

#[derive(Debug, Clone)]
pub struct CloudwatchQuery {
    pub datasource_uid: String,
    pub region: String,
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: BTreeMap<String, String>,
    pub statistic: String,
    pub period: u32,
}

#[async_trait::async_trait]
impl MetricsProvider for GrafanaClient {
    type Query = CloudwatchQuery;

    async fn query(
        &self,
        query: &Self::Query,
        range: MetricRange,
    ) -> MetricsResult<Vec<MetricSeries>> {
        self.fetch(
            GrafanaQueryRequest {
                ref_id: QUERY_REF_ID,
                datasource: GrafanaDatasource {
                    kind: "cloudwatch",
                    uid: &query.datasource_uid,
                },
                query_mode: "Metrics",
                metric_query_type: 0,
                metric_editor_mode: 0,
                region: &query.region,
                namespace: &query.namespace,
                metric_name: &query.metric_name,
                dimensions: query
                    .dimensions
                    .iter()
                    .map(|(name, value)| (name.as_str(), vec![value.as_str()]))
                    .collect(),
                statistic: &query.statistic,
                period: query.period.to_string(),
                match_exact: true,
                id: "",
                expression: "",
            },
            range,
        )
        .await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaQueryRequest<'a> {
    ref_id: &'static str,
    datasource: GrafanaDatasource<'a>,
    query_mode: &'static str,
    metric_query_type: u8,
    metric_editor_mode: u8,
    region: &'a str,
    namespace: &'a str,
    metric_name: &'a str,
    dimensions: BTreeMap<&'a str, Vec<&'a str>>,
    statistic: &'a str,
    period: String,
    match_exact: bool,
    id: &'static str,
    expression: &'static str,
}

#[derive(Debug, Serialize)]
struct GrafanaDatasource<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    uid: &'a str,
}
