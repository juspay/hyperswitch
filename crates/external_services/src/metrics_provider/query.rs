use time::PrimitiveDateTime;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricRange {
    pub from: PrimitiveDateTime,
    pub to: PrimitiveDateTime,
}
