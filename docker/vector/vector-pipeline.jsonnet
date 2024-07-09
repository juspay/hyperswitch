local vector = (import 'vendor/vector_jsonnet/vector.libsonnet').vector;
local log_config = import 'log-config.json';

vector
.global({
  acknowledgements: { enabled: true },
  api: { enabled: true },
})
.components({
  // Ingest
  kafka_source: vector.sources.kafka({
    bootstrap_servers: 'kafka0:29092',
    group_id: 'sessionizer',
    topics: ['hyperswitch-consolidated-events'],
    decoding: { codec: 'json' },
  }),

  init_sessionizer_vars: vector.transforms.remap({
    drop_on_error: true,
    reroute_dropped: true,
    source: |||
      .sessionizer = {
        "table": "${CASSANDRA_TABLE:-payments}",
        "id": string!(.log.merchant_id) + "_" + string!(.log.payment_id),
        "db_log_type": "payment_intent"
      };
      log({"log": ., "type": "individual"}, rate_limit_secs:0);
    |||,
  }),

  buffer_each_log_type: vector.transforms.reduce({
    group_by: ['log.log_type', 'log.merchant_id', 'log.payment_id', 'log.attempt_id', 'log.refund_id', 'log.dispute_id'],
    merge_strategies: {
      log: 'retain',
    },
    expire_after_ms: 3000,
  }),

  concat_all_log_types: vector.transforms.reduce({
    group_by: ['log.merchant_id', 'log.payment_id'],
    merge_strategies: {
      log: 'array',
      log_type: 'array',
    },
    expire_after_ms: 5000,
  }),

  debug_log: vector.transforms.remap({
    drop_on_error: true,
    reroute_dropped: true,
    source: |||
      log({"log": ., "type": "combined"}, rate_limit_secs:0);
    |||,
  }),

  generate_signed_events: vector.transforms.remap({
    drop_on_error: true,
    reroute_dropped: true,
    source: |||
      events = [];
      if exists(.old_log) {
        .old_log = parse_json(.old_log) ?? {};
        .old_log.sign_flag = -1;
        events = push(events, .old_log);
      };
      .log.sign_flag = 1;
      events = push(events, .log);
      . = events
    |||,
  }),
})
.pipelines([
  ['kafka_source', 'regex_parser', 'log_to_metric', 'console_metrics'],
  ['kafka_source', 'regex_parser', 'log_to_metric', 'prometheus'],
  ['kafka_source', 'regex_parser', 'console_logs'],
])
.json
