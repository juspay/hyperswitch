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

  cassandra_sessionize: vector.transforms.lua({
    version: '2',
    hooks: {
      init: 'init',
      process: 'process',
      shutdown: 'shutdown',
    },
    source: |||
      local socket = require "socket"
      local cassandra = require "cassandra"
      local json = require "json"

      function sleep(sec)
        socket.select(nil, nil, sec)
      end

      function client_connect()
        assert(client:connect())
      end

      function client_close()
        assert(client:close())
      end

      function create_cassandra_connection()
        client = assert(cassandra.new {
          host = os.getenv("CASSANDRA_HOST") or "cassandra0",
          port = os.getenv("CASSANDRA_PORT") or 9042,
          keyspace = os.getenv("CASSANDRA_KEYSPACE") or "sessionizer",
          auth = cassandra.auth_providers.plain_text(os.getenv("CASSANDRA_USERNAME") or "cassandra", os.getenv("CASSANDRA_PASSWORD") or "cassandra"),
          ssl = os.getenv("CASSANDRA_SSL") or false,
          cert = os.getenv("CASSANDRA_CERT") or ""
        })

        client:settimeout(1000)
        client:setkeepalive(0)
        client_connect()
      end

      function db_get(table, id, version)
        local version = version or 1
        local rows = assert(client:execute(string.format("SELECT * FROM %s WHERE id = ?", table), {
          id
        }))
        return rows
      end

      function db_set(table, state, id, version)
        version = version or 1
        local res = assert(client:execute(string.format("INSERT INTO %s (id, state, version) VALUES (?, ?, ?)", table), {
          id,
          state,
          version
        },{consistency = cassandra.consistencies.local_quorum}))
        return res
      end

      function db_get_wait(table, id, version)
        local is_success, data = pcall(db_get, table, id, version)
        while not is_success do
          sleep(1)

          print(string.format("db_get - recreate cassandra connection start - reason - %s", data))
          local connection_closed, err = pcall(client_close)
          print(string.format("db_get - close old cassandra connection - result - %s, %s", connection_closed, err))
          local connection_success, err = pcall(create_cassandra_connection)
          print(string.format("db_get - recreate cassandra connection end - result - %s, %s", connection_success, err))

          is_success, data = pcall(db_get, table, id, version)
        end

        return data
      end

      function db_set_wait(table, state, id, version)
        local is_success, res = pcall(db_set, table, state, id, version)

        while not is_success do
          sleep(1)

          print(string.format("db_set - recreate cassandra connection start - reason - %s", res))
          local connection_closed, err = pcall(client_close)
          print(string.format("db_set - close old cassandra connection - result - %s, %s", connection_closed, err))
          local connection_success, err = pcall(create_cassandra_connection)
          print(string.format("db_set - recreate cassandra connection end - result - %s, %s", connection_success, err))

          is_success, res = pcall(db_set, table, state, id, version)
        end

        return res
      end

      function get_value(tb, path)
        local value = tb

        for i, v in ipairs(path) do
          value = value[v] or {}
        end

        return value
      end

      function set_value(tb, path, value, index)
        local index = index or 1

        if index > #path then
          return value
        else
          tb[path[index]] = set_value(tb[path[index]] or {}, path, value, index + 1)
        end
        
        return tb
      end

      function merge_log(db_log, new_log, traverse_map)
        local log = get_value(db_log, traverse_map)
        local db_modified_at = log["modified_at"] or 0
        local new_modified_at = new_log["modified_at"] or 0
        if(new_modified_at > db_modified_at) then
          for k, v in pairs(new_log) do
            log[k] = v
          end
        end
        return set_value(db_log, traverse_map, log)
      end

      function init(emit) 
        create_cassandra_connection()  
      end

      function process(event, emit)
        local start_time = os.clock()
        local sessionizer = event.log.sessionizer
        local new_log_types = event.log.log_type
        local new_logs = event.log.log
        local db_log = {}

        local data = db_get_wait(sessionizer.table, sessionizer.id)

        if #data == 1 then
          event.log.old_log = data[1].state
          db_log = json.decode(data[1].state)
        end
        

        for i, new_log in ipairs(new_logs) do
          new_log_type = new_log_types[i];

          local traverse_map = nil
          if new_log_type == "payment_intent" then
            traverse_map = {}
          elseif new_log_type == "payment_attempt" then
            traverse_map = {"attempts", new_log.attempt_id}
          elseif new_log_type == "refund" then
            traverse_map = {"attempts", new_log.attempt_id, "refunds", new_log.refund_id}
          elseif new_log_type == "dispute" then
            traverse_map = {"attempts", new_log.attempt_id, "disputes", new_log.dispute_id}
          end

          db_log = merge_log(db_log, new_log, traverse_map);
        end

        event.log.log = db_log

        local _new_event_insert = db_set_wait(sessionizer.table, json.encode(db_log), sessionizer.id)
        emit(event)

        print(string.format("lua execution time: %.3f", os.clock() - start_time))
      end

      function shutdown(emit)
        client:close()
      end
    |||,
  }),
})
.pipelines([
  ['kafka_source', 'regex_parser', 'log_to_metric', 'console_metrics'],
  ['kafka_source', 'regex_parser', 'log_to_metric', 'prometheus'],
  ['kafka_source', 'regex_parser', 'console_logs'],
])
.json
