pub struct Milliseconds {
    pub milliseconds: u64,
}

pub struct SchedulerOptions {
    pub looper_interval: Milliseconds,
    pub db_name: String,
    pub cache_name: String,
    pub schema_name: String,
    pub cache_expiry: Milliseconds,
    pub runners: Vec<String>,
    pub fetch_limit: i32,
    pub fetch_limit_product_factor: i32,
    pub query_order: String,
    pub readiness: ReadinessOptions,
}

pub struct ReadinessOptions {
    pub is_ready: bool,
    pub graceful_termination_duration: Milliseconds,
}
