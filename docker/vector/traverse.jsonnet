local log_config = import 'log-config.json';
local vector_config = import 'vector-config.json';

local pass_down_init(obj, level, level_log) =
  '{' +
  std.foldl(function(acc, key) acc + std.format('"%s_%s":%s.%s, ', [level, key, level_log, obj[key]]), std.objectFields(obj), '')
  + '}'
;

local either_set(obj_name, log_name) =
  std.format(|||
    %s = merge(%s, %s);
  |||, [log_name, log_name, obj_name])
;

local roll_up_init(obj, level) =
  '{' +
  std.foldl(function(acc, key) acc + std.format('"%s_%s":0, ', [level, key]), std.objectFields(obj), '')
  + '}'
;

local roll_up_get(obj, roll_up_name, level, level_log) =
  std.foldl(
    function(acc, key)
      local roll_up = obj[key];
      local roll_up_key = '%s.%s_%s' % [roll_up_name, level_log, key];
      acc +
      if roll_up.kind == 'count' then
        std.format(|||
          %s = %s + 1;
        |||, [roll_up_key, roll_up_key])
      else if roll_up.kind == 'sum' then
        std.format(|||
          %s = %s + to_int(%s.%s) ?? 0;
        |||, [roll_up_key, roll_up_key, level, roll_up.field])
    ,
    std.objectFields(obj),
    ''
  )
;

local generate_vrl(obj) =
  local current_level = obj.level;
  local level_log = '%s_log' % current_level;
  local pass_down_var = 'pass_down_%s' % [current_level];
  (if std.length(obj.children) > 0 then
     std.format(|||
       %s = %s;
       pass_down = merge(pass_down, %s);
     |||, [pass_down_var, pass_down_init(obj.pass_down, current_level, level_log), pass_down_var])
   else '')
  +
  std.foldl(
    function(acc, child)
      local child_level_log = '%s_log' % child.level;
      local pass_down_set = either_set('pass_down', child_level_log);
      local child_accessor = '%s.%s' % [level_log, child.log_label];
      local child_obj = '%s = object(%s) ?? {};' % [child.log_label, child_accessor];
      local child_modify = '%s = set!(%s, [%s], %s);' % [child_accessor, child_accessor, child.id, child_level_log];
      local roll_up_var = 'roll_up_%s' % [child.level];
      local roll_up = std.format(|||
        %s = %s;
        roll_up = merge(roll_up, %s);
      |||, [roll_up_var, roll_up_init(child.roll_up, child.level), roll_up_var]);
      acc +
      std.format(|||
        %s
        %s
        for_each(%s) -> |%s, %s| {
            %s = object(%s) ?? {};
            %s
            %s
            %s
            %s
        };
      |||, [roll_up, child_obj, child.log_label, child.id, child.level, child_level_log, child.level, pass_down_set, roll_up_get(child.roll_up, 'roll_up', child_level_log, child.level), generate_vrl(child), child_modify])
    ,
    obj.children,
    ''
  ) +
  (if std.length(obj.children) > 0 then
     std.format(|||
       %s
     |||, [either_set('roll_up', level_log)])
   else '')
;

local create_vrl(obj) =
  local level_log = '%s_log' % obj.level;
  std.format(|||
    roll_up = {};
    pass_down = {};
    %s = .;
  |||, [level_log]) +
  generate_vrl(obj)
  +
  std.format(|||
    . = %s;
  |||, [level_log])
;

local prefix_log(arr) =
  std.map(function(v) 'log.' + v, arr)
;

local top_level_keys(config, obj) =
  config.top_level_additional_ids + [obj.id]
;

local init_sessionizer_vars(config, obj) =
  local sessionizer_id = std.join(' + "_" + ', std.map(function(x) 'string!(.%s)' % x, prefix_log(top_level_keys(config, obj))));
  std.format(|||
    .sessionizer = {
      "table": "${CASSANDRA_TABLE:-payments}",
      "id": %s,
      "db_log_type": "payment_intent"
    };
    log({"log": ., "type": "individual"}, rate_limit_secs:0);
  |||, [sessionizer_id])
;

// local consolidate_events(config, obj, level, prev_level_ids = []) =
//   local ids = top_level_keys(config, obj) + prev_level_ids;
//   local sessionizer_id = std.join(' + "_" + ', std.map(function(x) 'string!(%s)' % x, prefix_log(top_level_keys(config, obj))));
//   local sessionizer_id_exists = std.join(' && ', std.map(function(x) 'exists(%s)' % x, prefix_log(top_level_keys(config, obj))));
//   local delete_children =  std.join('', std.map(function(x) 'del(%s);\n' % x, prefix_log(std.map(function(child) child.level, obj.children))));
//   if obj.level == level then
//     std.format(|||
//       if %s {
//         %s
//         .sessionizer_key = %s;
//         log
//       } else {
//         []
//       }
//     |||, [sessionizer_id_exists, delete_children, sessionizer_id])
//   else
//     local depth_search_res = std.filter(
//       function(val) val != null,
//       std.map(
//         function(child)
//           local val = consolidate_events(config, child, level, prev_level_ids = prev_level_ids + [obj.id]);
//           local obj_level_log = '%s_log' % obj.level;
//           local child_level_log = '%s_log' % child.level;
//           local child_contents = "%s.%s" % [obj_level_log, child.log_label];
//           if val != null then
//             std.format(|||
//               map_values(values(object(%s) ?? {})) -> |%s| {
//                 %s = object(%s) ?? {};
//                 %s
//               }
//             |||, [child_contents, child.level, child_level_log, child_level, val])

//           ,
//         obj.children
//       )
//     );
//     if std.length(depth_search_res) == 0 then
//       null
//     else
//       depth_search_res[0]
//     std.format(|||
//       . = flatten(map_values(values(object(.attempts) ?? {})) -> |a| {
//           attempt = object(a) ?? {};
//           map_values(values(object(attempt.refunds) ?? {})) -> |r| {
//             refund = object(r) ?? {};
//             if exists(refund.merchant_id) && exists(refund.payment_id) && exists(refund.attempt_id) && exists(refund.refund_id) {
//               refund.sign_flag = .sign_flag;
//               # refund.log_type = "refund";
//               refund.sessionizer_key = string!(refund.merchant_id) + "_" + string!(refund.payment_id) + "_" + string!(refund.attempt_id) + "_" + string!(refund.refund_id);
//               refund
//             } else {
//               []
//             }
//           }
//         })
//     |||, [sessionizer_id])
// ;


//  { [obj.level]: key } +
//   if std.length(obj.children) == 0 then
//     {}
//   else
//     std.foldl(function(acc, child) acc + traverse(child, key=key + [child.log_label]), obj.children, {})


init_sessionizer_vars(vector_config, log_config)

// create_vrl(log_config)
