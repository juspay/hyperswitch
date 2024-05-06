
--- A builtin build system: back-end to provide a portable way of building C-based Lua modules.
local builtin = {}

-- This build driver checks LUA_INCDIR and LUA_LIBDIR on demand,
-- so that pure-Lua rocks don't need to have development headers
-- installed.
builtin.skip_lua_inc_lib_check = true

local unpack = unpack or table.unpack
local dir_sep = package.config:sub(1, 1)

local fs = require("luarocks.fs")
local path = require("luarocks.path")
local util = require("luarocks.util")
local cfg = require("luarocks.core.cfg")
local dir = require("luarocks.dir")
local deps = require("luarocks.deps")

local function autoextract_libs(external_dependencies, variables)
   if not external_dependencies then
      return nil, nil, nil
   end
   local libs = {}
   local incdirs = {}
   local libdirs = {}
   for name, data in pairs(external_dependencies) do
      if data.library then
         table.insert(libs, data.library)
         table.insert(incdirs, variables[name .. "_INCDIR"])
         table.insert(libdirs, variables[name .. "_LIBDIR"])
      end
   end
   return libs, incdirs, libdirs
end

do
   local function get_cmod_name(file)
      local fd = io.open(dir.path(fs.current_dir(), file), "r")
      if not fd then return nil end
      local data = fd:read("*a")
      fd:close()
      return (data:match("int%s+luaopen_([a-zA-Z0-9_]+)"))
   end

   local skiplist = {
      ["spec"] = true,
      [".luarocks"] = true,
      ["lua_modules"] = true,
      ["test.lua"] = true,
      ["tests.lua"] = true,
   }

   function builtin.autodetect_modules(libs, incdirs, libdirs)
      local modules = {}
      local install
      local copy_directories

      local prefix = ""
      for _, parent in ipairs({"src", "lua", "lib"}) do
         if fs.is_dir(parent) then
            fs.change_dir(parent)
            prefix = parent .. dir_sep
            break
         end
      end

      for _, file in ipairs(fs.find()) do
         local base = file:match("^([^\\/]*)")
         if not skiplist[base] then
            local luamod = file:match("(.*)%.lua$")
            if luamod then
               modules[path.path_to_module(file)] = prefix .. file
            else
               local cmod = file:match("(.*)%.c$")
               if cmod then
                  local modname = get_cmod_name(file) or path.path_to_module(file:gsub("%.c$", ".lua"))
                  modules[modname] = {
                     sources = prefix..file,
                     libraries = libs,
                     incdirs = incdirs,
                     libdirs = libdirs,
                  }
               end
            end
         end
      end

      if prefix ~= "" then
         fs.pop_dir()
      end

      local bindir = (fs.is_dir(dir.path("src", "bin")) and dir.path("src", "bin"))
                  or (fs.is_dir("bin") and "bin")
      if bindir then
         install = { bin = {} }
         for _, file in ipairs(fs.list_dir(bindir)) do
            table.insert(install.bin, dir.path(bindir, file))
         end
      end

      for _, directory in ipairs({ "doc", "docs", "samples", "tests" }) do
         if fs.is_dir(directory) then
            if not copy_directories then
               copy_directories = {}
            end
            table.insert(copy_directories, directory)
         end
      end

      return modules, install, copy_directories
   end
end

--- Run a command displaying its execution on standard output.
-- @return boolean: true if command succeeds (status code 0), false
-- otherwise.
local function execute(...)
   io.stdout:write(table.concat({...}, " ").."\n")
   return fs.execute(...)
end

--- Driver function for the builtin build back-end.
-- @param rockspec table: the loaded rockspec.
-- @return boolean or (nil, string): true if no errors occurred,
-- nil and an error message otherwise.
function builtin.run(rockspec, no_install)
   assert(rockspec:type() == "rockspec")
   local compile_object, compile_library, compile_static_library

   local build = rockspec.build
   local variables = rockspec.variables
   local checked_lua_h = false

   for _, var in ipairs{ "CC", "CFLAGS", "LDFLAGS" } do
      variables[var] = variables[var] or os.getenv(var) or ""
   end

   local function add_flags(extras, flag, flags)
      if flags then
         if type(flags) ~= "table" then
            flags = { tostring(flags) }
         end
         util.variable_substitutions(flags, variables)
         for _, v in ipairs(flags) do
            table.insert(extras, flag:format(v))
         end
      end
   end

   if cfg.is_platform("mingw32") then
      compile_object = function(object, source, defines, incdirs)
         local extras = {}
         add_flags(extras, "-D%s", defines)
         add_flags(extras, "-I%s", incdirs)
         return execute(variables.CC.." "..variables.CFLAGS, "-c", "-o", object, "-I"..variables.LUA_INCDIR, source, unpack(extras))
      end
      compile_library = function(library, objects, libraries, libdirs, name)
         local extras = { unpack(objects) }
         add_flags(extras, "-L%s", libdirs)
         add_flags(extras, "-l%s", libraries)
         extras[#extras+1] = dir.path(variables.LUA_LIBDIR, variables.LUALIB)

         if variables.CC == "clang" or variables.CC == "clang-cl" then
            local exported_name = name:gsub("%.", "_")
            exported_name = exported_name:match('^[^%-]+%-(.+)$') or exported_name
            extras[#extras+1] = string.format("-Wl,-export:luaopen_%s", exported_name)
         else
            extras[#extras+1] = "-l" .. (variables.MSVCRT or "m")
         end

         local ok = execute(variables.LD.." "..variables.LDFLAGS.." "..variables.LIBFLAG, "-o", library, unpack(extras))
         return ok
      end
      --[[ TODO disable static libs until we fix the conflict in the manifest, which will take extending the manifest format.
      compile_static_library = function(library, objects, libraries, libdirs, name)
         local ok = execute(variables.AR, "rc", library, unpack(objects))
         if ok then
            ok = execute(variables.RANLIB, library)
         end
         return ok
      end
      ]]
   elseif cfg.is_platform("win32") then
      compile_object = function(object, source, defines, incdirs)
         local extras = {}
         add_flags(extras, "-D%s", defines)
         add_flags(extras, "-I%s", incdirs)
         return execute(variables.CC.." "..variables.CFLAGS, "-c", "-Fo"..object, "-I"..variables.LUA_INCDIR, source, unpack(extras))
      end
      compile_library = function(library, objects, libraries, libdirs, name)
         local extras = { unpack(objects) }
         add_flags(extras, "-libpath:%s", libdirs)
         add_flags(extras, "%s.lib", libraries)
         local basename = dir.base_name(library):gsub(".[^.]*$", "")
         local deffile = basename .. ".def"
         local def = io.open(dir.path(fs.current_dir(), deffile), "w+")
         local exported_name = name:gsub("%.", "_")
         exported_name = exported_name:match('^[^%-]+%-(.+)$') or exported_name
         def:write("EXPORTS\n")
         def:write("luaopen_"..exported_name.."\n")
         def:close()
         local ok = execute(variables.LD, "-dll", "-def:"..deffile, "-out:"..library, dir.path(variables.LUA_LIBDIR, variables.LUALIB), unpack(extras))
         local basedir = ""
         if name:find("%.") ~= nil then
            basedir = name:gsub("%.%w+$", "\\")
            basedir = basedir:gsub("%.", "\\")
         end
         local manifestfile = basedir .. basename..".dll.manifest"

         if ok and fs.exists(manifestfile) then
            ok = execute(variables.MT, "-manifest", manifestfile, "-outputresource:"..basedir..basename..".dll;2")
         end
         return ok
      end
      --[[ TODO disable static libs until we fix the conflict in the manifest, which will take extending the manifest format.
      compile_static_library = function(library, objects, libraries, libdirs, name)
         local ok = execute(variables.AR, "-out:"..library, unpack(objects))
         return ok
      end
      ]]
   else
      compile_object = function(object, source, defines, incdirs)
         local extras = {}
         add_flags(extras, "-D%s", defines)
         add_flags(extras, "-I%s", incdirs)
         return execute(variables.CC.." "..variables.CFLAGS, "-I"..variables.LUA_INCDIR, "-c", source, "-o", object, unpack(extras))
      end
      compile_library = function (library, objects, libraries, libdirs)
         local extras = { unpack(objects) }
         add_flags(extras, "-L%s", libdirs)
         if cfg.gcc_rpath then
            add_flags(extras, "-Wl,-rpath,%s", libdirs)
         end
         add_flags(extras, "-l%s", libraries)
         if cfg.link_lua_explicitly then
            extras[#extras+1] = "-L"..variables.LUA_LIBDIR
            extras[#extras+1] = "-llua"
         end
         return execute(variables.LD.." "..variables.LDFLAGS.." "..variables.LIBFLAG, "-o", library, unpack(extras))
      end
      compile_static_library = function(library, objects, libraries, libdirs, name)  -- luacheck: ignore 211
         local ok = execute(variables.AR, "rc", library, unpack(objects))
         if ok then
            ok = execute(variables.RANLIB, library)
         end
         return ok
      end
   end

   local ok, err
   local lua_modules = {}
   local lib_modules = {}
   local luadir = path.lua_dir(rockspec.name, rockspec.version)
   local libdir = path.lib_dir(rockspec.name, rockspec.version)

   local autolibs, autoincdirs, autolibdirs = autoextract_libs(rockspec.external_dependencies, rockspec.variables)

   if not build.modules then
      if rockspec:format_is_at_least("3.0") then
         local install, copy_directories
         build.modules, install, copy_directories = builtin.autodetect_modules(autolibs, autoincdirs, autolibdirs)
         build.install = build.install or install
         build.copy_directories = build.copy_directories or copy_directories
      else
         return nil, "Missing build.modules table"
      end
   end

   local compile_temp_dir

   local mkdir_cache = {}
   local function cached_make_dir(name)
      if name == "" or mkdir_cache[name] then
         return true
      end
      mkdir_cache[name] = true
      return fs.make_dir(name)
   end

   for name, info in pairs(build.modules) do
      local moddir = path.module_to_path(name)
      if type(info) == "string" then
         local ext = info:match("%.([^.]+)$")
         if ext == "lua" then
            local filename = dir.base_name(info)
            if filename == "init.lua" and not name:match("%.init$") then
               moddir = path.module_to_path(name..".init")
            else
               local basename = name:match("([^.]+)$")
               filename = basename..".lua"
            end
            local dest = dir.path(luadir, moddir, filename)
            lua_modules[info] = dest
         else
            info = {info}
         end
      end
      if type(info) == "table" then
         if not checked_lua_h then
            local ok, err, errcode = deps.check_lua_incdir(rockspec.variables)
            if not ok then
               return nil, err, errcode
            end

            if cfg.link_lua_explicitly then
               local ok, err, errcode = deps.check_lua_libdir(rockspec.variables)
               if not ok then
                  return nil, err, errcode
               end
            end
            checked_lua_h = true
         end
         local objects = {}
         local sources = info.sources
         if info[1] then sources = info end
         if type(sources) == "string" then sources = {sources} end
         if type(sources) ~= "table" then
            return nil, "error in rockspec: module '" .. name .. "' entry has no 'sources' list"
         end
         for _, source in ipairs(sources) do
            if type(source) ~= "string" then
               return nil, "error in rockspec: module '" .. name .. "' does not specify source correctly."
            end
            local object = source:gsub("%.[^.]*$", "."..cfg.obj_extension)
            if not object then
               object = source.."."..cfg.obj_extension
            end
            ok = compile_object(object, source, info.defines, info.incdirs or autoincdirs)
            if not ok then
               return nil, "Failed compiling object "..object
            end
            table.insert(objects, object)
         end

         if not compile_temp_dir then
            compile_temp_dir = fs.make_temp_dir("build-" .. rockspec.package .. "-" .. rockspec.version)
            util.schedule_function(fs.delete, compile_temp_dir)
         end

         local module_name = name:match("([^.]*)$").."."..util.matchquote(cfg.lib_extension)
         if moddir ~= "" then
            module_name = dir.path(moddir, module_name)
         end

         local build_name = dir.path(compile_temp_dir, module_name)
         local build_dir = dir.dir_name(build_name)
         cached_make_dir(build_dir)

         lib_modules[build_name] = dir.path(libdir, module_name)
         ok = compile_library(build_name, objects, info.libraries, info.libdirs or autolibdirs, name)
         if not ok then
            return nil, "Failed compiling module "..module_name
         end

         -- for backwards compatibility, try keeping a copy of the module
         -- in the old location (luasec-1.3.2-1 rockspec breaks otherwise)
         if cached_make_dir(dir.dir_name(module_name)) then
            fs.copy(build_name, module_name)
         end

         --[[ TODO disable static libs until we fix the conflict in the manifest, which will take extending the manifest format.
         module_name = name:match("([^.]*)$").."."..util.matchquote(cfg.static_lib_extension)
         if moddir ~= "" then
            module_name = dir.path(moddir, module_name)
         end
         lib_modules[module_name] = dir.path(libdir, module_name)
         ok = compile_static_library(module_name, objects, info.libraries, info.libdirs, name)
         if not ok then
            return nil, "Failed compiling static library "..module_name
         end
         ]]
      end
   end
   if not no_install then
      for _, mods in ipairs({{ tbl = lua_modules, perms = "read" }, { tbl = lib_modules, perms = "exec" }}) do
         for name, dest in pairs(mods.tbl) do
            cached_make_dir(dir.dir_name(dest))
            ok, err = fs.copy(name, dest, mods.perms)
            if not ok then
               return nil, "Failed installing "..name.." in "..dest..": "..err
            end
         end
      end
      if fs.is_dir("lua") then
         ok, err = fs.copy_contents("lua", luadir)
         if not ok then
            return nil, "Failed copying contents of 'lua' directory: "..err
         end
      end
   end
   return true
end

return builtin
