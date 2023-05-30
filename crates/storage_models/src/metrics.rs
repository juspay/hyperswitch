use router_env::{counter_metric, global_meter, histogram_metric, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

histogram_metric!(DB_REQUEST_TIME, GLOBAL_METER);
counter_metric!(DB_REQUEST_COUNT, GLOBAL_METER);

pub mod database_metric {
    use std::{future::Future, time::Instant};

    use router_env::opentelemetry::KeyValue;

    use super::*;
    #[derive(Debug)]
    pub enum DatabaseCallType {
        Create,
        Read,
        Update,
        Delete,
    }

    impl DatabaseCallType {
        fn as_str(&self) -> &'static str {
            match self {
                Self::Create => "create",
                Self::Read => "read",
                Self::Update => "update",
                Self::Delete => "delete",
            }
        }
    }

    pub async fn time_database_call<F: FnOnce() -> Fut, Fut: Future<Output = R>, R>(
        call_type: DatabaseCallType,
        future: F,
        table: impl ToString,
    ) -> R {
        let start = Instant::now();
        let output = future().await;
        let end_time = start.elapsed().as_secs_f64();
        DB_REQUEST_COUNT.add(
            &CONTEXT,
            1,
            &[
                KeyValue::new("call_type", call_type.as_str()),
                KeyValue::new("table", table.to_string()),
            ],
        );
        DB_REQUEST_TIME.record(
            &CONTEXT,
            end_time,
            &[
                KeyValue::new("call_type", call_type.as_str()),
                KeyValue::new("table", table.to_string()),
            ],
        );

        output
    }
}
