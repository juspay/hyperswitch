use time::PrimitiveDateTime;

#[derive(Debug, Clone, PartialEq)]
pub struct MetricSeries {
    pub points: Vec<MetricPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub timestamp: PrimitiveDateTime,
    pub value: f64,
}
