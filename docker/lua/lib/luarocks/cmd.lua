
--- Functions for command-line scripts.
local cmd = {}

local manif = require("luarocks.manif")
local config = require("luarocks.config")
local util = require("luarocks.util")
local path = require("luarocks.path")
local cfg = require("luarocks.core.cfg")
local dir = require("luarocks.dir")
local fun = require("luarocks.fun")
local fs = require("luarocks.fs")
local argparse = require("luarocks.vendor.argparse")

local unpack = table.unpack or unpack
local pack = table.pack or function(...) return { n = select("#", ...), ... } end

local hc_ok, hardcoded = pcall(require, "luarocks.core.hardcoded")
if not hc_ok then
   hardcoded = {}
end

local program = util.this_program("luarocks")

cmd.errorcodes = {
   OK = 0,
   UNSPECIFIED = 1,
   PERMISSIONDENIED = 2,
   CONFIGFILE = 3,
   LOCK = 4,
   CRASH = 99
}

local function check_popen()
   local popen_ok, popen_result = pcall(io.popen, "")
   if popen_ok then
      if popen_result then
         popen_result:close()
      end
   else
      io.stderr:write("Your version of Lua does not support io.popen,\n")
      io.stderr:write("which is required by LuaRocks. Please check your Lua installation.\n")
      os.exit(cmd.errorcodes.UNSPECIFIED)
   end
end

local process_tree_args
do
   local function replace_tree(args, root, tree)
      root = dir.normalize(root)
      args.tree = root
      path.use_tree(tree or root)
   end

   local function strip_trailing_slashes()
      if type(cfg.root_dir) == "string" then
        cfg.root_dir = cfg.root_dir:gsub("/+$", "")
      else
        cfg.root_dir.root = cfg.root_dir.root:gsub("/+$", "")
      end
      cfg.rocks_dir = cfg.rocks_dir:gsub("/+$", "")
      cfg.deploy_bin_dir = cfg.deploy_bin_dir:gsub("/+$", "")
      cfg.deploy_lua_dir = cfg.deploy_lua_dir:gsub("/+$", "")
      cfg.deploy_lib_dir = cfg.deploy_lib_dir:gsub("/+$", "")
   end

   local function set_named_tree(args, name)
      for _, tree in ipairs(cfg.rocks_trees) do
         if type(tree) == "table" and name == tree.name then
            if not tree.root then
               return nil, "Configuration error: tree '"..tree.name.."' has no 'root' field."
            end
            replace_tree(args, tree.root, tree)
            return true
         end
      end
      return false
   end

   process_tree_args = function(args, project_dir)

      if args.global then
         local ok, err = set_named_tree(args, "system")
         if not ok then
            return nil, err
         end
      elseif args.tree then
         local named = set_named_tree(args, args.tree)
         if not named then
            local root_dir = fs.absolute_name(args.tree)
            replace_tree(args, root_dir)
            if (args.deps_mode or cfg.deps_mode) ~= "order" then
               table.insert(cfg.rocks_trees, 1, { name = "arg", root = root_dir } )
            end
         end
      elseif args["local"] then
         if fs.is_superuser() then
            return nil, "The --local flag is meant for operating in a user's home directory.\n"..
               "You are running as a superuser, which is intended for system-wide operation.\n"..
               "To force using the superuser's home, use --tree explicitly."
         else
            local ok, err = set_named_tree(args, "user")
            if not ok then
               return nil, err
            end
         end
      elseif args.project_tree then
         local tree = args.project_tree
         table.insert(cfg.rocks_trees, 1, { name = "project", root = tree } )
         manif.load_rocks_tree_manifests()
         path.use_tree(tree)
      elseif cfg.local_by_default then
         local ok, err = set_named_tree(args, "user")
         if not ok then
            return nil, err
         end
      elseif project_dir then
         local project_tree = project_dir .. "/lua_modules"
         table.insert(cfg.rocks_trees, 1, { name = "project", root = project_tree } )
         manif.load_rocks_tree_manifests()
         path.use_tree(project_tree)
      else
         local trees = cfg.rocks_trees
         path.use_tree(trees[#trees])
      end

      strip_trailing_slashes()

      cfg.variables.ROCKS_TREE = cfg.rocks_dir
      cfg.variables.SCRIPTS_DIR = cfg.deploy_bin_dir

      return true
   end
end

local function process_server_args(args)
   if args.server then
      local protocol, pathname = dir.split_url(args.server)
      table.insert(cfg.rocks_servers, 1, protocol.."://"..pathname)
   end

   if args.dev then
      local append_dev = function(s) return dir.path(s, "dev") end
      local dev_servers = fun.traverse(cfg.rocks_servers, append_dev)
      cfg.rocks_servers = fun.concat(dev_servers, cfg.rocks_servers)
   end

   if args.only_server then
      if args.dev then
         return nil, "--only-server cannot be used with --dev"
      end
      if args.server then
         return nil, "--only-server cannot be used with --server"
      end
      cfg.rocks_servers = { args.only_server }
   end

   return true
end

local function error_handler(err)
   if not debug then
      return err
   end
   local mode = "Arch.: " .. (cfg and cfg.arch or "unknown")
   if package.config:sub(1, 1) == "\\" then
      if cfg and cfg.fs_use_modules then
         mode = mode .. " (fs_use_modules = true)"
      end
   end
   if cfg and cfg.is_binary then
      mode = mode .. " (binary)"
   end
   return debug.traceback("LuaRocks "..cfg.program_version..
      " bug (please report at https://github.com/luarocks/luarocks/issues).\n"..
      mode.."\n"..err, 2)
end

--- Display an error message and exit.
-- @param message string: The error message.
-- @param exitcode number: the exitcode to use
local function die(message, exitcode)
   assert(type(message) == "string", "bad error, expected string, got: " .. type(message))
   assert(exitcode == nil or type(exitcode) == "number", "bad error, expected number, got: " .. type(exitcode) .. " - " .. tostring(exitcode))
   util.printerr("\nError: "..message)

   local ok, err = xpcall(util.run_scheduled_functions, error_handler)
   if not ok then
      util.printerr("\nError: "..err)
      exitcode = cmd.errorcodes.CRASH
   end

   os.exit(exitcode or cmd.errorcodes.UNSPECIFIED)
end

local function search_lua(lua_version, verbose, search_at)
   if search_at then
      return util.find_lua(search_at, lua_version, verbose)
   end

   local path_sep = (package.config:sub(1, 1) == "\\" and ";" or ":")
   local all_tried = {}
   for bindir in (os.getenv("PATH") or ""):gmatch("[^"..path_sep.."]+") do
      local searchdir = (bindir:gsub("[\\/]+bin[\\/]?$", ""))
      local detected, tried = util.find_lua(searchdir, lua_version)
      if detected then
         return detected
      else
         table.insert(all_tried, tried)
      end
   end
   return nil, "Could not find " ..
               (lua_version and "Lua " .. lua_version or "Lua") ..
               " in PATH." ..
               (verbose and " Tried:\n" .. table.concat(all_tried, "\n") or "")
end

local init_config
do
   local detect_config_via_args
   do
      local function find_project_dir(project_tree)
         if project_tree then
            return project_tree:gsub("[/\\][^/\\]+$", ""), true
         else
            local try = "."
            for _ = 1, 10 do -- FIXME detect when root dir was hit instead
               if util.exists(try .. "/.luarocks") and util.exists(try .. "/lua_modules") then
                  return dir.normalize(try), false
               elseif util.exists(try .. "/.luarocks-no-project") then
                  break
               end
               try = try .. "/.."
            end
         end
         return nil
      end

      local function find_default_lua_version(args, project_dir)
         if hardcoded.FORCE_CONFIG then
            return nil
         end

         local dirs = {}
         if project_dir then
            table.insert(dirs, dir.path(project_dir, ".luarocks"))
         end
         if cfg.homeconfdir then
            table.insert(dirs, cfg.homeconfdir)
         end
         table.insert(dirs, cfg.sysconfdir)
         for _, d in ipairs(dirs) do
            local f = dir.path(d, "default-lua-version.lua")
            local mod, err = loadfile(f, "t")
            if mod then
               local pok, ver = pcall(mod)
               if pok and type(ver) == "string" and ver:match("%d+.%d+") then
                  if args.verbose then
                     util.printout("Defaulting to Lua " .. ver .. " based on " .. f .. " ...")
                  end
                  return ver
               end
            end
         end
         return nil
      end

      local function find_version_from_config(dirname)
         return fun.find(util.lua_versions("descending"), function(v)
            if util.exists(dir.path(dirname, ".luarocks", "config-"..v..".lua")) then
               return v
            end
         end)
      end

      local function detect_lua_via_args(args, project_dir)
         local lua_version = args.lua_version
                             or find_default_lua_version(args, project_dir)
                             or (project_dir and find_version_from_config(project_dir))

         if args.lua_dir then
            local detected, err = util.find_lua(args.lua_dir, lua_version)
            if not detected then
               local suggestion = (not args.lua_version)
                  and "\nYou may want to specify a different Lua version with --lua-version\n"
                  or  ""
               die(err .. suggestion)
            end
            return detected
         end

         if lua_version then
            local detected = search_lua(lua_version)
            if detected then
               return detected
            end
            return {
               lua_version = lua_version,
            }
         end

         return {}
      end

      detect_config_via_args = function(args)
         local project_dir, given
         if not args.no_project then
            project_dir, given = find_project_dir(args.project_tree)
         end

         local detected = detect_lua_via_args(args, project_dir)
         if args.lua_version then
            detected.given_lua_version = args.lua_version
         end
         if args.lua_dir then
            detected.given_lua_dir = args.lua_dir
         end
         if given then
            detected.given_project_dir = project_dir
         end
         detected.project_dir = project_dir
         return detected
      end
   end

   init_config = function(args)
      local detected = detect_config_via_args(args)

      local ok, err = cfg.init(detected, util.warning)
      if not ok then
         return nil, err
      end

      return (detected.lua_dir ~= nil)
   end
end

local variables_help = [[
Variables:
   Variables from the "variables" table of the configuration file can be
   overridden with VAR=VALUE assignments.

]]

local lua_example = package.config:sub(1, 1) == "\\"
                    and "<d:\\path\\lua.exe>"
                    or  "</path/lua>"

local function show_status(file, status, err)
   return (file and file .. " " or "") .. (status and "(ok)" or ("(" .. (err or "not found") ..")"))
end

local function use_to_fix_location(key, what)
   local buf =  "                   ****************************************\n"
   buf = buf .. "                   Use the command\n\n"
   buf = buf .. "                      luarocks config " .. key .. " " .. (what or "<dir>") .. "\n\n"
   buf = buf .. "                   to fix the location\n"
   buf = buf .. "                   ****************************************\n"
   return buf
end

local function get_config_text(cfg)  -- luacheck: ignore 431
   local deps = require("luarocks.deps")

   local libdir_ok = deps.check_lua_libdir(cfg.variables)
   local incdir_ok = deps.check_lua_incdir(cfg.variables)
   local lua_ok = cfg.variables.LUA and fs.exists(cfg.variables.LUA)

   local buf = "Configuration:\n"
   buf = buf.."   Lua:\n"
   buf = buf.."      Version    : "..cfg.lua_version.."\n"
   if cfg.luajit_version then
      buf = buf.."      LuaJIT     : "..cfg.luajit_version.."\n"
   end
   buf = buf.."      LUA        : "..show_status(cfg.variables.LUA, lua_ok, "interpreter not found").."\n"
   if not lua_ok then
      buf = buf .. use_to_fix_location("variables.LUA", lua_example)
   end
   buf = buf.."      LUA_INCDIR : "..show_status(cfg.variables.LUA_INCDIR, incdir_ok, "lua.h not found").."\n"
   if lua_ok and not incdir_ok then
      buf = buf .. use_to_fix_location("variables.LUA_INCDIR")
   end
   buf = buf.."      LUA_LIBDIR : "..show_status(cfg.variables.LUA_LIBDIR, libdir_ok, "Lua library itself not found").."\n"
   if lua_ok and not libdir_ok then
      buf = buf .. use_to_fix_location("variables.LUA_LIBDIR")
   end

   buf = buf.."\n   Configuration files:\n"
   local conf = cfg.config_files
   buf = buf.."      System  : "..show_status(fs.absolute_name(conf.system.file), conf.system.found).."\n"
   if conf.user.file then
      buf = buf.."      User    : "..show_status(fs.absolute_name(conf.user.file), conf.user.found).."\n"
   else
      buf = buf.."      User    : disabled in this LuaRocks installation.\n"
   end
   if conf.project then
      buf = buf.."      Project : "..show_status(fs.absolute_name(conf.project.file), conf.project.found).."\n"
   end
   buf = buf.."\n   Rocks trees in use: \n"
   for _, tree in ipairs(cfg.rocks_trees) do
      if type(tree) == "string" then
         buf = buf.."      "..fs.absolute_name(tree)
      else
         local name = tree.name and " (\""..tree.name.."\")" or ""
         buf = buf.."      "..fs.absolute_name(tree.root)..name
      end
      buf = buf .. "\n"
   end

   return buf
end

local function get_parser(description, cmd_modules)
   local basename = dir.base_name(program)
   local parser = argparse(
      basename, "LuaRocks "..cfg.program_version..", the Lua package manager\n\n"..
      program.." - "..description, variables_help.."Run '"..basename..
      "' without any arguments to see the configuration.")
      :help_max_width(80)
      :add_help_command()
      :add_complete_command({
         help_max_width = 100,
         summary = "Output a shell completion script.",
         description = [[
Output a shell completion script.

Enabling completions for Bash:

   Add the following line to your ~/.bashrc:
      source <(]]..basename..[[ completion bash)
   or save the completion script to the local completion directory:
      ]]..basename..[[ completion bash > ~/.local/share/bash-completion/completions/]]..basename..[[


Enabling completions for Zsh:

   Save the completion script to a file in your $fpath.
   You can add a new directory to your $fpath by adding e.g.
      fpath=(~/.zfunc $fpath)
   to your ~/.zshrc.
   Then run:
      ]]..basename..[[ completion zsh > ~/.zfunc/_]]..basename..[[


Enabling completion for Fish:

   Add the following line to your ~/.config/fish/config.fish:
      ]]..basename..[[ completion fish | source
   or save the completion script to the local completion directory:
      ]]..basename..[[ completion fish > ~/.config/fish/completions/]]..basename..[[.fish
]]})
      :command_target("command")
      :require_command(false)

   parser:flag("--version", "Show version info and exit.")
      :action(function()
         util.printout(program.." "..cfg.program_version)
         util.printout(description)
         util.printout()
         os.exit(cmd.errorcodes.OK)
      end)
   parser:flag("--dev", "Enable the sub-repositories in rocks servers for "..
      "rockspecs of in-development versions.")
   parser:option("--server", "Fetch rocks/rockspecs from this server "..
      "(takes priority over config file).")
      :hidden_name("--from")
   parser:option("--only-server", "Fetch rocks/rockspecs from this server only "..
      "(overrides any entries in the config file).")
      :argname("<server>")
      :hidden_name("--only-from")
   parser:option("--only-sources", "Restrict downloads to paths matching the given URL.")
      :argname("<url>")
      :hidden_name("--only-sources-from")
   parser:option("--namespace", "Specify the rocks server namespace to use.")
      :convert(string.lower)
   parser:option("--lua-dir", "Which Lua installation to use.")
      :argname("<prefix>")
   parser:option("--lua-version", "Which Lua version to use.")
      :argname("<ver>")
      :convert(function(s) return (s:match("^%d+%.%d+$")) end)
   parser:option("--tree", "Which tree to operate on.")
      :hidden_name("--to")
   parser:flag("--local", "Use the tree in the user's home directory.\n"..
      "To enable it, see '"..program.." help path'.")
   parser:flag("--global", "Use the system tree when `local_by_default` is `true`.")
   parser:flag("--no-project", "Do not use project tree even if running from a project folder.")
   parser:flag("--force-lock", "Attempt to overwrite the lock for commands " ..
      "that require exclusive access, such as 'install'")
   parser:flag("--verbose", "Display verbose output of commands executed.")
   parser:option("--timeout", "Timeout on network operations, in seconds.\n"..
      "0 means no timeout (wait forever). Default is "..
      tostring(cfg.connection_timeout)..".")
      :argname("<seconds>")
      :convert(tonumber)

   -- Used internally to force the use of a particular project tree
   parser:option("--project-tree"):hidden(true)

   for _, module in util.sortedpairs(cmd_modules) do
      module.add_to_parser(parser)
   end

   return parser
end

local function get_first_arg()
   if not arg then
      return
   end
   local first_arg = arg[0]
   local i = -1
   while arg[i] do
      first_arg = arg[i]
      i = i -1
   end
   return first_arg
end

--- Main command-line processor.
-- Parses input arguments and calls the appropriate driver function
-- to execute the action requested on the command-line, forwarding
-- to it any additional arguments passed by the user.
-- @param description string: Short summary description of the program.
-- @param commands table: contains the loaded modules representing commands.
-- @param external_namespace string: where to look for external commands.
-- @param ... string: Arguments given on the command-line.
function cmd.run_command(description, commands, external_namespace, ...)

   check_popen()

   -- Preliminary initialization
   cfg.init()

   fs.init()

   for _, module_name in ipairs(fs.modules(external_namespace)) do
      if not commands[module_name] then
         commands[module_name] = external_namespace.."."..module_name
      end
   end

   local cmd_modules = {}
   for name, module in pairs(commands) do
      local pok, mod = pcall(require, module)
      if pok and type(mod) == "table" then
         local original_command = mod.command
         if original_command then
            if not mod.add_to_parser then
               mod.add_to_parser = function(parser)
                  parser:command(name, mod.help, util.see_also())
                        :summary(mod.help_summary)
                        :handle_options(false)
                        :argument("input")
                        :args("*")
               end

               mod.command = function(args)
                  return original_command(args, unpack(args.input))
               end
            end
            cmd_modules[name] = mod
         else
            util.warning("command module " .. module .. " does not implement command(), skipping")
         end
      else
         util.warning("failed to load command module " .. module .. ": " .. mod)
      end
   end

   local function process_cmdline_vars(...)
      local args = pack(...)
      local cmdline_vars = {}
      local last = args.n
      for i = 1, args.n do
         if args[i] == "--" then
            last = i - 1
            break
         end
      end
      for i = last, 1, -1 do
         local arg = args[i]
         if arg:match("^[^-][^=]*=") then
            local var, val = arg:match("^([A-Z_][A-Z0-9_]*)=(.*)")
            if val then
               cmdline_vars[var] = val
               table.remove(args, i)
            else
               die("Invalid assignment: "..arg)
            end
         end
      end

      return args, cmdline_vars
   end

   local args, cmdline_vars = process_cmdline_vars(...)
   local parser = get_parser(description, cmd_modules)
   args = parser:parse(args)

   -- Compatibility for old flag
   if args.nodeps then
      args.deps_mode = "none"
   end

   if args.timeout then -- setting it in the config file will kick-in earlier in the process
      cfg.connection_timeout = args.timeout
   end

   if args.command == "config" then
      if args.key == "lua_version" and args.value then
         args.lua_version = args.value
      elseif args.key == "lua_dir" and args.value then
         args.lua_dir = args.value
      end
   end

   -----------------------------------------------------------------------------
   local lua_found, err = init_config(args)
   if err then
      die(err)
   end
   -----------------------------------------------------------------------------

   -- Now that the config is fully loaded, reinitialize fs using the full
   -- feature set.
   fs.init()

   -- if the Lua interpreter wasn't explicitly found before cfg.init,
   -- try again now.
   local tried
   if not lua_found then
      local detected
      detected, tried = search_lua(cfg.lua_version, args.verbose, cfg.variables.LUA_DIR)
      if detected then
         lua_found = true
         cfg.variables.LUA = detected.lua
         cfg.variables.LUA_DIR = detected.lua_dir
         cfg.variables.LUA_BINDIR = detected.lua_bindir
         if args.lua_dir then
            cfg.variables.LUA_INCDIR = nil
            cfg.variables.LUA_LIBDIR = nil
         end
      else
         cfg.variables.LUA = nil
         cfg.variables.LUA_DIR = nil
         cfg.variables.LUA_BINDIR = nil
         cfg.variables.LUA_INCDIR = nil
         cfg.variables.LUA_LIBDIR = nil
      end
   end

   if lua_found then
      assert(cfg.variables.LUA)
   else
      -- Fallback producing _some_ Lua configuration based on the running interpreter.
      -- Most likely won't produce correct results when running from the standalone binary,
      -- so eventually we need to drop this and outright fail if Lua is not found
      -- or explictly configured
      if not cfg.variables.LUA then
         local first_arg = get_first_arg()
         local bin_dir = dir.dir_name(fs.absolute_name(first_arg))
         local exe = dir.base_name(first_arg)
         exe = exe:match("rocks") and ("lua" .. (cfg.arch:match("win") and ".exe" or "")) or exe
         local full_path = dir.path(bin_dir, exe)
         if util.check_lua_version(full_path, cfg.lua_version) then
            cfg.variables.LUA = dir.path(bin_dir, exe)
            cfg.variables.LUA_DIR = bin_dir:gsub("[/\\]bin[/\\]?$", "")
            cfg.variables.LUA_BINDIR = bin_dir
            cfg.variables.LUA_INCDIR = nil
            cfg.variables.LUA_LIBDIR = nil
         end
      end
   end

   cfg.lua_found = lua_found

   if cfg.project_dir then
      cfg.project_dir = fs.absolute_name(cfg.project_dir)
   end

   if args.verbose then
      cfg.verbose = true
      print(("-"):rep(79))
      print("Current configuration:")
      print(("-"):rep(79))
      print(config.to_string(cfg))
      print(("-"):rep(79))
      fs.verbose()
   end

   if (not fs.current_dir()) or fs.current_dir() == "" then
      die("Current directory does not exist. Please run LuaRocks from an existing directory.")
   end

   local ok, err = process_tree_args(args, cfg.project_dir)
   if not ok then
      die(err)
   end

   ok, err = process_server_args(args)
   if not ok then
      die(err)
   end

   if args.only_sources then
      cfg.only_sources_from = args.only_sources
   end

   for k, v in pairs(cmdline_vars) do
      cfg.variables[k] = v
   end

   -- if running as superuser, use system cache dir
   if fs.is_superuser() then
      cfg.local_cache = dir.path(fs.system_cache_dir(), "luarocks")
   end

   if args.no_manifest then
      cfg.no_manifest = true
   end

   if not args.command then
      parser:epilog(variables_help..get_config_text(cfg))
      util.printout()
      util.printout(parser:get_help())
      util.printout()
      os.exit(cmd.errorcodes.OK)
   end

   if not cfg.variables["LUA"] and args.command ~= "config" and args.command ~= "help" then
      local flag = (not cfg.project_tree)
                   and "--local "
                   or ""
      if args.lua_version then
         flag = "--lua-version=" .. args.lua_version .. " " .. flag
      end
      die((tried or "Lua interpreter not found.") ..
         "\nPlease set your Lua interpreter with:\n\n" ..
         "   luarocks " .. flag.. "config variables.LUA " .. lua_example .. "\n")
   end

   local cmd_mod = cmd_modules[args.command]

   local lock
   if cmd_mod.needs_lock and cmd_mod.needs_lock(args) then
      local ok, err = fs.check_command_permissions(args)
      if not ok then
         die(err, cmd.errorcodes.PERMISSIONDENIED)
      end

      lock, err = fs.lock_access(path.root_dir(cfg.root_dir), args.force_lock)
      if not lock then
         err = args.force_lock
               and ("failed to force the lock" .. (err and ": " .. err or ""))
               or  (err and err ~= "File exists")
                   and err
                   or  "try --force-lock to overwrite the lock"

         die("command '" .. args.command .. "' " ..
             "requires exclusive write access to " .. path.root_dir(cfg.root_dir) .. " - " ..
             err, cmd.errorcodes.LOCK)
      end
   end

   local call_ok, ok, err, exitcode = xpcall(function()
      return cmd_mod.command(args)
   end, error_handler)

   if lock then
      fs.unlock_access(lock)
   end

   if not call_ok then
      die(ok, cmd.errorcodes.CRASH)
   elseif not ok then
      die(err, exitcode)
   end
   util.run_scheduled_functions()
end

return cmd
