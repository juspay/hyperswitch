local log_config = import 'log-config.json';

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

create_vrl(log_config)
