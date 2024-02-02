impl Default for super::settings::SchedulerSettings {
        /// This method creates a new instance of the struct with default values for its fields.
    fn default() -> Self {
        Self {
            stream: "SCHEDULER_STREAM".into(),
            producer: super::settings::ProducerSettings::default(),
            consumer: super::settings::ConsumerSettings::default(),
            graceful_shutdown_interval: 60000,
            loop_interval: 5000,
            server: super::settings::Server::default(),
        }
    }
}

impl Default for super::settings::ProducerSettings {
        /// Returns a new instance of the struct with default values for the upper and lower fetch limits, lock key, lock time-to-live, and batch size.
    fn default() -> Self {
        Self {
            upper_fetch_limit: 0,
            lower_fetch_limit: 1800,
            lock_key: "PRODUCER_LOCKING_KEY".into(),
            lock_ttl: 160,
            batch_size: 200,
        }
    }
}

impl Default for super::settings::ConsumerSettings {
        /// Creates a new instance of the struct with default values.
    fn default() -> Self {
        Self {
            disabled: false,
            consumer_group: "SCHEDULER_GROUP".into(),
        }
    }
}

impl Default for super::settings::Server {
    fn default() -> Self {
        Self {
            port: 8080,
            workers: num_cpus::get_physical(),
            host: "localhost".into(),
        }
    }
}
