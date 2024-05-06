--- Module implementing the LuaRocks "config" command.
-- Queries information about the LuaRocks configuration.
local config_cmd = {}

local persist = require("luarocks.persist")
local config = require("luarocks.config")
local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")
local deps = require("luarocks.deps")
local dir = require("luarocks.dir")
local fs = require("luarocks.fs")
local json = require("luarocks.vendor.dkjson")

function config_cmd.add_to_parser(parser)
   local cmd = parser:command("config", [[
Query information about the LuaRocks configuration.

* When given a configuration key, it prints the value of that key according to
  the currently active configuration (taking into account all config files and
  any command-line flags passed)

  Examples:
     luarocks config variables.LUA_INCDIR
     luarocks config lua_version

* When given a configuration key and a value, it overwrites the config file (see
  the --scope option below to determine which) and replaces the value of the
  given key with the given value.

  * `lua_dir` is a special key as it checks for a valid Lua installation
    (equivalent to --lua-dir) and sets several keys at once.
  * `lua_version` is a special key as it changes the default Lua version
    used by LuaRocks commands (equivalent to passing --lua-version).

  Examples:
     luarocks config variables.OPENSSL_DIR /usr/local/openssl
     luarocks config lua_dir /usr/local
     luarocks config lua_version 5.3

* When given a configuration key and --unset, it overwrites the config file (see
  the --scope option below to determine which) and deletes that key from the
  file.

  Example: luarocks config variables.OPENSSL_DIR --unset

* When given no arguments, it prints the entire currently active configuration,
  resulting from reading the config files from all scopes.

  Example: luarocks config]], util.see_also([[
   https://github.com/luarocks/luarocks/wiki/Config-file-format
   for detailed information on the LuaRocks config file format.
]]))
      :summary("Query information about the LuaRocks configuration.")

   cmd:argument("key", "The configuration key.")
      :args("?")
   cmd:argument("value", "The configuration value.")
      :args("?")

   cmd:option("--scope", "The scope indicates which config file should be rewritten.\n"..
      '* Using a wrapper created with `luarocks init`, the default is "project".\n'..
      '* Using --local (or when `local_by_default` is `true`), the default is "user".\n'..
      '* Otherwise, the default is "system".')
      :choices({"system", "user", "project"})
   cmd:flag("--unset", "Delete the key from the configuration file.")
   cmd:flag("--json", "Output as JSON.")

   -- Deprecated flags
   cmd:flag("--lua-incdir"):hidden(true)
   cmd:flag("--lua-libdir"):hidden(true)
   cmd:flag("--lua-ver"):hidden(true)
   cmd:flag("--system-config"):hidden(true)
   cmd:flag("--user-config"):hidden(true)
   cmd:flag("--rock-trees"):hidden(true)
end

local function config_file(conf)
   print(dir.normalize(conf.file))
   if conf.found then
      return true
   else
      return nil, "file not found"
   end
end

local function traverse_varstring(var, tbl, fn, missing_parent)
   local k, r = var:match("^%[([0-9]+)%]%.(.*)$")
   if k then
      k = tonumber(k)
   else
      k, r = var:match("^([^.[]+)%.(.*)$")
      if not k then
         k, r = var:match("^([^[]+)(%[.*)$")
      end
   end

   if k then
      if not tbl[k] and missing_parent then
         missing_parent(tbl, k)
      end

      if tbl[k] then
         return traverse_varstring(r, tbl[k], fn, missing_parent)
      else
         return nil, "Unknown entry " .. k
      end
   end

   local i = var:match("^%[([0-9]+)%]$")
   if i then
      var = tonumber(i)
   end

   return fn(tbl, var)
end

local function print_json(value)
   print(json.encode(value))
   return true
end

local function print_entry(var, tbl, is_json)
   return traverse_varstring(var, tbl, function(t, k)
      if not t[k] then
         return nil, "Unknown entry " .. k
      end
      local val = t[k]

      if not config.should_skip(var, val) then
         if is_json then
            return print_json(val)
         elseif type(val) == "string" then
            print(val)
         else
            persist.write_value(io.stdout, val)
         end
      end
      return true
   end)
end

local function infer_type(var)
   local typ
   traverse_varstring(var, cfg, function(t, k)
      if t[k] ~= nil then
         typ = type(t[k])
      end
   end)
   return typ
end

local function write_entries(keys, scope, do_unset)
   if scope == "project" and not cfg.config_files.project then
      return nil, "Current directory is not part of a project. You may want to run `luarocks init`."
   end

   local file_name = cfg.config_files[scope].file

   local tbl, err = persist.load_config_file_if_basic(file_name, cfg)
   if not tbl then
      return nil, err
   end

   for var, val in util.sortedpairs(keys) do
      traverse_varstring(var, tbl, function(t, k)
         if do_unset then
            t[k] = nil
         else
            local typ = infer_type(var)
            local v
            if typ == "number" and tonumber(val) then
               v = tonumber(val)
            elseif typ == "boolean" and val == "true" then
               v = true
            elseif typ == "boolean" and val == "false" then
               v = false
            else
               v = val
            end
            t[k] = v
            keys[var] = v
         end
         return true
      end, function(p, k)
         p[k] = {}
      end)
   end

   local ok, err = fs.make_dir(dir.dir_name(file_name))
   if not ok then
      return nil, err
   end

   ok, err = persist.save_from_table(file_name, tbl)
   if ok then
      print(do_unset and "Removed" or "Wrote")
      for var, val in util.sortedpairs(keys) do
         if do_unset then
            print(("\t%s"):format(var))
         else
            if type(val) == "string" then
               print(("\t%s = %q"):format(var, val))
            else
               print(("\t%s = %s"):format(var, tostring(val)))
            end
         end
      end
      print(do_unset and "from" or "to")
      print("\t" .. file_name)
      return true
   else
      return nil, err
   end
end

local function get_scope(args)
   return args.scope
          or (args["local"] and "user")
          or (args.project_tree and "project")
          or (cfg.local_by_default and "user")
          or (fs.is_writable(cfg.config_files["system"].file) and "system")
          or "user"
end

local function report_on_lua_incdir_config(value, lua_version)
   local variables = {
      ["LUA_DIR"] = cfg.variables.LUA_DIR,
      ["LUA_BINDIR"] = cfg.variables.LUA_BINDIR,
      ["LUA_INCDIR"] = value,
      ["LUA_LIBDIR"] = cfg.variables.LUA_LIBDIR,
      ["LUA"] = cfg.variables.LUA,
   }

   local ok, err = deps.check_lua_incdir(variables, lua_version)
   if not ok then
      util.printerr()
      util.warning((err:gsub(" You can use.*", "")))
   end
   return ok
end

local function report_on_lua_libdir_config(value, lua_version)
   local variables = {
      ["LUA_DIR"] = cfg.variables.LUA_DIR,
      ["LUA_BINDIR"] = cfg.variables.LUA_BINDIR,
      ["LUA_INCDIR"] = cfg.variables.LUA_INCDIR,
      ["LUA_LIBDIR"] = value,
      ["LUA"] = cfg.variables.LUA,
   }

   local ok, err, _, err_files = deps.check_lua_libdir(variables, lua_version)
   if not ok then
      util.printerr()
      util.warning((err:gsub(" You can use.*", "")))
      util.printerr("Tried:")
      for _, l in pairs(err_files) do
         for _, d in ipairs(l) do
            util.printerr("\t" .. d)
         end
      end
   end
   return ok
end

local function warn_bad_c_config()
   util.printerr()
   util.printerr("LuaRocks may not work correctly when building C modules using this configuration.")
   util.printerr()
end

--- Driver function for "config" command.
-- @return boolean: True if succeeded, nil on errors.
function config_cmd.command(args)
   local lua_version = args.lua_version or cfg.lua_version

   deps.check_lua_incdir(cfg.variables, lua_version)
   deps.check_lua_libdir(cfg.variables, lua_version)

   -- deprecated flags
   if args.lua_incdir then
      print(cfg.variables.LUA_INCDIR)
      return true
   end
   if args.lua_libdir then
      print(cfg.variables.LUA_LIBDIR)
      return true
   end
   if args.lua_ver then
      print(cfg.lua_version)
      return true
   end
   if args.system_config then
      return config_file(cfg.config_files.system)
   end
   if args.user_config then
      return config_file(cfg.config_files.user)
   end
   if args.rock_trees then
      for _, tree in ipairs(cfg.rocks_trees) do
      	if type(tree) == "string" then
      	   util.printout(dir.normalize(tree))
      	else
      	   local name = tree.name and "\t"..tree.name or ""
      	   util.printout(dir.normalize(tree.root)..name)
      	end
      end
      return true
   end

   if args.key == "lua_version" and args.value then
      local scope = get_scope(args)
      if scope == "project" and not cfg.config_files.project then
         return nil, "Current directory is not part of a project. You may want to run `luarocks init`."
      end

      local location = cfg.config_files[scope]
      if (not location) or (not location.file) then
         return nil, "could not get config file location for " .. tostring(scope) .. " scope"
      end

      local prefix = dir.dir_name(location.file)
      local ok, err = persist.save_default_lua_version(prefix, args.value)
      if not ok then
         return nil, "could not set default Lua version: " .. err
      end
      print("Lua version will default to " .. args.value .. " in " .. prefix)
   end

   if args.key == "lua_dir" and args.value then
      local scope = get_scope(args)
      local keys = {
         ["variables.LUA_DIR"] = cfg.variables.LUA_DIR,
         ["variables.LUA_BINDIR"] = cfg.variables.LUA_BINDIR,
         ["variables.LUA_INCDIR"] = cfg.variables.LUA_INCDIR,
         ["variables.LUA_LIBDIR"] = cfg.variables.LUA_LIBDIR,
         ["variables.LUA"] = cfg.variables.LUA,
      }
      if args.lua_version then
         local prefix = dir.dir_name(cfg.config_files[scope].file)
         persist.save_default_lua_version(prefix, args.lua_version)
      end
      local ok, err = write_entries(keys, scope, args.unset)
      if ok then
         local inc_ok = report_on_lua_incdir_config(cfg.variables.LUA_INCDIR, lua_version)
         local lib_ok = ok and report_on_lua_libdir_config(cfg.variables.LUA_LIBDIR, lua_version)
         if not (inc_ok and lib_ok) then
            warn_bad_c_config()
         end
      end

      return ok, err
   end

   if args.key then
      if args.key:match("^[A-Z]") then
         args.key = "variables." .. args.key
      end

      if args.value or args.unset then
         local scope = get_scope(args)

         local ok, err = write_entries({ [args.key] = args.value or args.unset }, scope, args.unset)

         if ok then
            if args.key == "variables.LUA_INCDIR" then
               local ok = report_on_lua_incdir_config(args.value, lua_version)
               if not ok then
                  warn_bad_c_config()
               end
            elseif args.key == "variables.LUA_LIBDIR" then
               local ok = report_on_lua_libdir_config(args.value, lua_version)
               if not ok then
                  warn_bad_c_config()
               end
            end
         end

         return ok, err
      else
         return print_entry(args.key, cfg, args.json)
      end
   end

   if args.json then
      return print_json(config.get_config_for_display(cfg))
   else
      print(config.to_string(cfg))
      return true
   end
end

return config_cmd
