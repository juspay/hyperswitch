local vector = (import 'vendor/vector_jsonnet/vector.libsonnet').vector;
local log_config = import 'log-config.json';

vector
.global({
  acknowledgements: { enabled: true },
  api: { enabled: true },
})
.components({
  // Ingest
  kafka_source: vector.sources.file({
    include: ['sample.log'],
    start_at_beginning: true,
  }),

  // Structure and parse the data
  regex_parser: vector.transforms.remap({
    drop_on_error: false,
    source: |||
      . |= parse_regex!(.message, r'^(?P<host>[\w\.]+) - (?P<user>[\w-]+) \[(?P<timestamp>.*)\] "(?P<method>[\w]+) (?P<path>.*)" (?P<status>[\d]+) (?P<bytes_out>[\d]+)$'),
    |||,
  }),

  // Transform into metrics
  log_to_metric: vector.transforms.log_to_metric({
    metrics: [
      { type: 'counter', field: 'message' },
      { type: 'counter', field: 'bytes_out', name: 'bytes_out_total', increment_by_value: true },
      { type: 'gauge', field: 'bytes_out' },
      { type: 'set', field: 'user' },
      { type: 'histogram', field: 'bytes_out', name: 'bytes_out_histogram' },
    ],
  }),

  // Output data
  console_metrics: vector.sinks.console({ encoding: { codec: 'json' } }),
  console_logs: vector.sinks.console({ encoding: { codec: 'text' } }),
  prometheus: vector.sinks.prometheus_exporter({
    default_namespace: 'vector',
    buckets: [0.0, 10.0, 100.0, 1000.0, 10000.0, 100001.0],
  }),
})
.pipelines([
  ['file', 'regex_parser', 'log_to_metric', 'console_metrics'],
  ['file', 'regex_parser', 'log_to_metric', 'prometheus'],
  ['file', 'regex_parser', 'console_logs'],
])
.json
