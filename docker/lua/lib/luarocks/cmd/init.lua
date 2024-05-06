
local init = {}

local cfg = require("luarocks.core.cfg")
local fs = require("luarocks.fs")
local path = require("luarocks.path")
local deps = require("luarocks.deps")
local dir = require("luarocks.dir")
local util = require("luarocks.util")
local persist = require("luarocks.persist")
local write_rockspec = require("luarocks.cmd.write_rockspec")

function init.add_to_parser(parser)
   local cmd = parser:command("init", "Initialize a directory for a Lua project using LuaRocks.", util.see_also())

   cmd:argument("name", "The project name.")
      :args("?")
   cmd:argument("version", "An optional project version.")
      :args("?")
   cmd:option("--wrapper-dir", "Location where the 'lua' and 'luarocks' wrapper scripts " ..
      "should be generated; if not given, the current directory is used as a default.")
   cmd:flag("--reset", "Delete any .luarocks/config-5.x.lua and ./lua and generate new ones.")
   cmd:flag("--no-wrapper-scripts", "Do not generate wrapper ./lua and ./luarocks launcher scripts.")
   cmd:flag("--no-gitignore", "Do not generate a .gitignore file.")

   cmd:group("Options for specifying rockspec data", write_rockspec.cmd_options(cmd))
end

local function gitignore_path(pwd, wrapper_dir, filename)
   local norm_cur = fs.absolute_name(pwd)
   local norm_file = fs.absolute_name(dir.path(wrapper_dir, filename))
   if norm_file:sub(1, #norm_cur) == norm_cur then
      return norm_file:sub(#norm_cur + 2)
   else
      return filename
   end
end

local function write_gitignore(entries)
   local gitignore = ""
   local fd = io.open(".gitignore", "r")
   if fd then
      gitignore = fd:read("*a")
      fd:close()
      gitignore = "\n" .. gitignore .. "\n"
   end

   fd = io.open(".gitignore", gitignore and "a" or "w")
   for _, entry in ipairs(entries) do
      entry = "/" .. entry
      if not gitignore:find("\n"..entry.."\n", 1, true) then
         fd:write(entry.."\n")
      end
   end
   fd:close()
end

local function inject_tree(tree)
   path.use_tree(tree)
   local tree_set = false
   for _, t in ipairs(cfg.rocks_trees) do
      if type(t) == "table" then
         if t.name == "project" then
            t.root = tree
            tree_set = true
         end
      end
   end
   if not tree_set then
      table.insert(cfg.rocks_trees, 1, { name = "project", root = tree })
   end
end

local function write_wrapper_scripts(wrapper_dir, luarocks_wrapper, lua_wrapper)
   local tree = dir.path(fs.current_dir(), "lua_modules")

   fs.make_dir(wrapper_dir)

   luarocks_wrapper = dir.path(wrapper_dir, luarocks_wrapper)
   if not fs.exists(luarocks_wrapper) then
      util.printout("Preparing " .. luarocks_wrapper .. " ...")
      fs.wrap_script(arg[0], luarocks_wrapper, "none", nil, nil, "--project-tree", tree)
   else
      util.printout(luarocks_wrapper .. " already exists. Not overwriting it!")
   end

   lua_wrapper = dir.path(wrapper_dir, lua_wrapper)
   local write_lua_wrapper = true
   if fs.exists(lua_wrapper) then
      if not util.lua_is_wrapper(lua_wrapper) then
         util.printout(lua_wrapper .. " already exists and does not look like a wrapper script. Not overwriting.")
         write_lua_wrapper = false
      end
   end

   if write_lua_wrapper then
      if util.check_lua_version(cfg.variables.LUA, cfg.lua_version) then
         util.printout("Preparing " .. lua_wrapper .. " for version " .. cfg.lua_version .. "...")

         -- Inject tree so it shows up as a lookup path in the wrappers
         inject_tree(tree)

         fs.wrap_script(nil, lua_wrapper, "all")
      else
         util.warning("No Lua interpreter detected for version " .. cfg.lua_version .. ". Not creating " .. lua_wrapper)
      end
   end
end

--- Driver function for "init" command.
-- @return boolean: True if succeeded, nil on errors.
function init.command(args)
   local do_gitignore = not args.no_gitignore
   local do_wrapper_scripts = not args.no_wrapper_scripts
   local wrapper_dir = args.wrapper_dir or "."

   local pwd = fs.current_dir()

   if not args.name then
      args.name = dir.base_name(pwd)
      if args.name == "/" then
         return nil, "When running from the root directory, please specify the <name> argument"
      end
   end

   util.title("Initializing project '" .. args.name .. "' for Lua " .. cfg.lua_version .. " ...")

   local ok, err = deps.check_lua_incdir(cfg.variables)
   if not ok then
      return nil, err
   end

   local has_rockspec = false
   for file in fs.dir() do
      if file:match("%.rockspec$") then
         has_rockspec = true
         break
      end
   end

   if not has_rockspec then
      args.version = args.version or "dev"
      args.location = pwd
      local ok, err = write_rockspec.command(args)
      if not ok then
         util.printerr(err)
      end
   end

   local ext = cfg.wrapper_suffix
   local luarocks_wrapper = "luarocks" .. ext
   local lua_wrapper = "lua" .. ext

   if do_gitignore then
      util.printout("Adding entries to .gitignore ...")
      local ignores = { "lua_modules", ".luarocks" }
      if do_wrapper_scripts then
         table.insert(ignores, 1, gitignore_path(pwd, wrapper_dir, luarocks_wrapper))
         table.insert(ignores, 2, gitignore_path(pwd, wrapper_dir, lua_wrapper))
      end
      write_gitignore(ignores)
   end

   util.printout("Preparing ./.luarocks/ ...")
   fs.make_dir(".luarocks")
   local config_file = ".luarocks/config-" .. cfg.lua_version .. ".lua"

   if args.reset then
      if do_wrapper_scripts then
         fs.delete(fs.absolute_name(dir.path(wrapper_dir, lua_wrapper)))
      end
      fs.delete(fs.absolute_name(config_file))
   end

   local config_tbl, err = persist.load_config_file_if_basic(config_file, cfg)
   if config_tbl then
      local varnames = {
         "LUA_DIR",
         "LUA_INCDIR",
         "LUA_LIBDIR",
         "LUA_BINDIR",
         "LUA",
      }
      for _, varname in ipairs(varnames) do
         if cfg.variables[varname] then
            config_tbl.variables = config_tbl.variables or {}
            config_tbl.variables[varname] = cfg.variables[varname]
         end
      end
      local ok, err = persist.save_from_table(config_file, config_tbl)
      if ok then
         util.printout("Wrote " .. config_file)
      else
         util.printout("Failed writing " .. config_file .. ": " .. err)
      end
   else
      util.printout("Will not attempt to overwrite " .. config_file)
   end

   ok, err = persist.save_default_lua_version(".luarocks", cfg.lua_version)
   if not ok then
      util.printout("Failed setting default Lua version: " .. err)
   end

   util.printout("Preparing ./lua_modules/ ...")
   fs.make_dir("lua_modules/lib/luarocks/rocks-" .. cfg.lua_version)

   if do_wrapper_scripts then
      write_wrapper_scripts(wrapper_dir, luarocks_wrapper, lua_wrapper)
   end

   return true
end

init.needs_lock = function() return true end

return init
