
--- High-level dependency related functions.
local deps = {}

local cfg = require("luarocks.core.cfg")
local manif = require("luarocks.manif")
local path = require("luarocks.path")
local dir = require("luarocks.dir")
local fun = require("luarocks.fun")
local util = require("luarocks.util")
local vers = require("luarocks.core.vers")
local queries = require("luarocks.queries")
local deplocks = require("luarocks.deplocks")

--- Generate a function that matches dep queries against the manifest,
-- taking into account rocks_provided, the list of versions to skip,
-- and the lockfile.
-- @param deps_mode "one", "none", "all" or "order"
-- @param rocks_provided a one-level table mapping names to versions,
-- listing rocks to consider provided by the VM
-- @param rocks_provided table: A table of auto-provided dependencies.
-- by this Lua implementation for the given dependency.
-- @param depskey key to use when matching the lockfile ("dependencies",
-- "build_dependencies", etc.)
-- @param skip_set a two-level table mapping names to versions to
-- boolean, listing rocks that should not be matched
-- @return function(dep): {string}, {string:string}, string, boolean
-- * array of matching versions
-- * map of versions to locations
-- * version matched via lockfile if any
-- * true if rock matched via rocks_provided
local function prepare_get_versions(deps_mode, rocks_provided, depskey, skip_set)
   assert(type(deps_mode) == "string")
   assert(type(rocks_provided) == "table")
   assert(type(depskey) == "string")
   assert(type(skip_set) == "table" or skip_set == nil)

   return function(dep)
      local versions, locations
      local provided = rocks_provided[dep.name]
      if provided then
         -- Provided rocks have higher priority than manifest's rocks.
         versions, locations = { provided }, {}
      else
         if deps_mode == "none" then
            deps_mode = "one"
         end
         versions, locations = manif.get_versions(dep, deps_mode)
      end

      if skip_set and skip_set[dep.name] then
         for i = #versions, 1, -1 do
            local v = versions[i]
            if skip_set[dep.name][v] then
               table.remove(versions, i)
            end
         end
      end

      local lockversion = deplocks.get(depskey, dep.name)

      return versions, locations, lockversion, provided ~= nil
   end
end

--- Attempt to match a dependency to an installed rock.
-- @param get_versions a getter function obtained via prepare_get_versions
-- @return (string, string, table) or (nil, nil, table):
-- 1. latest installed version of the rock matching the dependency
-- 2. location where the installed version is installed
-- 3. the 'dep' query table
-- 4. true if provided via VM
-- or
-- 1. nil
-- 2. nil
-- 3. either 'dep' or an alternative query to be used
-- 4. false
local function match_dep(dep, get_versions)
   assert(type(dep) == "table")
   assert(type(get_versions) == "function")

   local versions, locations, lockversion, provided = get_versions(dep)

   local latest_version
   local latest_vstring
   for _, vstring in ipairs(versions) do
      local version = vers.parse_version(vstring)
      if vers.match_constraints(version, dep.constraints) then
         if not latest_version or version > latest_version then
            latest_version = version
            latest_vstring = vstring
         end
      end
   end

   if lockversion and not locations[lockversion] then
      local latest_matching_msg = ""
      if latest_vstring and latest_vstring ~= lockversion then
         latest_matching_msg = " (latest matching is " .. latest_vstring .. ")"
      end
      util.printout("Forcing " .. dep.name .. " to pinned version " .. lockversion .. latest_matching_msg)
      return nil, nil, queries.new(dep.name, dep.namespace, lockversion)
   end

   return latest_vstring, locations[latest_vstring], dep, provided
end

local function match_all_deps(dependencies, get_versions)
   assert(type(dependencies) == "table")
   assert(type(get_versions) == "function")

   local matched, missing, no_upgrade = {}, {}, {}

   for _, dep in ipairs(dependencies) do
      local found, _, provided
      found, _, dep, provided = match_dep(dep, get_versions)
      if found then
         if not provided then
            matched[dep] = {name = dep.name, version = found}
         end
      else
         if dep.constraints[1] and dep.constraints[1].no_upgrade then
            no_upgrade[dep.name] = dep
         else
            missing[dep.name] = dep
         end
      end
   end
   return matched, missing, no_upgrade
end

--- Attempt to match dependencies of a rockspec to installed rocks.
-- @param dependencies table: The table of dependencies.
-- @param rocks_provided table: The table of auto-provided dependencies.
-- @param skip_set table or nil: Program versions to not use as valid matches.
-- Table where keys are program names and values are tables where keys
-- are program versions and values are 'true'.
-- @param deps_mode string: Which trees to check dependencies for
-- @return table, table, table: A table where keys are dependencies parsed
-- in table format and values are tables containing fields 'name' and
-- version' representing matches; a table of missing dependencies
-- parsed as tables; and a table of "no-upgrade" missing dependencies
-- (to be used in plugin modules so that a plugin does not force upgrade of
-- its parent application).
function deps.match_deps(dependencies, rocks_provided, skip_set, deps_mode)
   assert(type(dependencies) == "table")
   assert(type(rocks_provided) == "table")
   assert(type(skip_set) == "table" or skip_set == nil)
   assert(type(deps_mode) == "string")

   local get_versions = prepare_get_versions(deps_mode, rocks_provided, "dependencies", skip_set)
   return match_all_deps(dependencies, get_versions)
end

local function rock_status(dep, get_versions)
   assert(dep:type() == "query")
   assert(type(get_versions) == "function")

   local installed, _, _, provided = match_dep(dep, get_versions)
   local installation_type = provided and "provided by VM" or "installed"
   return installed and installed.." "..installation_type..": success" or "not installed"
end

--- Check depenendencies of a package and report any missing ones.
-- @param name string: package name.
-- @param version string: package version.
-- @param dependencies table: array of dependencies.
-- @param deps_mode string: Which trees to check dependencies for
-- @param rocks_provided table: A table of auto-dependencies provided
-- by this Lua implementation for the given dependency.
-- "one" for the current default tree, "all" for all trees,
-- "order" for all trees with priority >= the current default, "none" for no trees.
function deps.report_missing_dependencies(name, version, dependencies, deps_mode, rocks_provided)
   assert(type(name) == "string")
   assert(type(version) == "string")
   assert(type(dependencies) == "table")
   assert(type(deps_mode) == "string")
   assert(type(rocks_provided) == "table")

   if deps_mode == "none" then
      return
   end

   local get_versions = prepare_get_versions(deps_mode, rocks_provided, "dependencies")

   local first_missing_dep = true

   for _, dep in ipairs(dependencies) do
      local found, _
      found, _, dep = match_dep(dep, get_versions)
      if not found then
         if first_missing_dep then
            util.printout(("Missing dependencies for %s %s:"):format(name, version))
            first_missing_dep = false
         end

         util.printout(("   %s (%s)"):format(tostring(dep), rock_status(dep, get_versions)))
      end
   end
end

function deps.fulfill_dependency(dep, deps_mode, rocks_provided, verify, depskey)
   assert(dep:type() == "query")
   assert(type(deps_mode) == "string" or deps_mode == nil)
   assert(type(rocks_provided) == "table" or rocks_provided == nil)
   assert(type(verify) == "boolean" or verify == nil)
   assert(type(depskey) == "string")

   deps_mode = deps_mode or "all"
   rocks_provided = rocks_provided or {}

   local get_versions = prepare_get_versions(deps_mode, rocks_provided, depskey)

   local found, where
   found, where, dep = match_dep(dep, get_versions)
   if found then
      local tree_manifests = manif.load_rocks_tree_manifests(deps_mode)
      manif.scan_dependencies(dep.name, found, tree_manifests, deplocks.proxy(depskey))
      return true, found, where
   end

   local search = require("luarocks.search")
   local install = require("luarocks.cmd.install")

   local url, search_err = search.find_suitable_rock(dep)
   if not url then
      return nil, "Could not satisfy dependency "..tostring(dep)..": "..search_err
   end
   util.printout("Installing "..url)
   local install_args = {
      rock = url,
      deps_mode = deps_mode,
      namespace = dep.namespace,
      verify = verify,
   }
   local ok, install_err, errcode = install.command(install_args)
   if not ok then
      return nil, "Failed installing dependency: "..url.." - "..install_err, errcode
   end

   found, where = match_dep(dep, get_versions)
   assert(found)
   return true, found, where
end

local function check_supported_platforms(rockspec)
   if rockspec.supported_platforms and next(rockspec.supported_platforms) then
      local all_negative = true
      local supported = false
      for _, plat in pairs(rockspec.supported_platforms) do
         local neg
         neg, plat = plat:match("^(!?)(.*)")
         if neg == "!" then
            if cfg.is_platform(plat) then
               return nil, "This rockspec for "..rockspec.package.." does not support "..plat.." platforms."
            end
         else
            all_negative = false
            if cfg.is_platform(plat) then
               supported = true
               break
            end
         end
      end
      if supported == false and not all_negative then
         local plats = cfg.print_platforms()
         return nil, "This rockspec for "..rockspec.package.." does not support "..plats.." platforms."
      end
   end

   return true
end

--- Check dependencies of a rock and attempt to install any missing ones.
-- Packages are installed using the LuaRocks "install" command.
-- Aborts the program if a dependency could not be fulfilled.
-- @param rockspec table: A rockspec in table format.
-- @param depskey string: Rockspec key to fetch to get dependency table
-- ("dependencies", "build_dependencies", etc.).
-- @param deps_mode string
-- @param verify boolean
-- @param deplock_dir string: dirname of the deplock file
-- @return boolean or (nil, string, [string]): True if no errors occurred, or
-- nil and an error message if any test failed, followed by an optional
-- error code.
function deps.fulfill_dependencies(rockspec, depskey, deps_mode, verify, deplock_dir)
   assert(type(rockspec) == "table")
   assert(type(depskey) == "string")
   assert(type(deps_mode) == "string")
   assert(type(verify) == "boolean" or verify == nil)
   assert(type(deplock_dir) == "string" or deplock_dir == nil)

   local name = rockspec.name
   local version = rockspec.version
   local rocks_provided = rockspec.rocks_provided

   local ok, filename, err = deplocks.load(name, deplock_dir or ".")
   if filename then
      util.printout("Using dependencies pinned in lockfile: " .. filename)

      local get_versions = prepare_get_versions("none", rocks_provided, depskey)
      for dnsname, dversion in deplocks.each(depskey) do
         local dname, dnamespace = util.split_namespace(dnsname)
         local dep = queries.new(dname, dnamespace, dversion)

         util.printout(("%s %s is pinned to %s (%s)"):format(
            name, version, tostring(dep), rock_status(dep, get_versions)))

         local ok, err = deps.fulfill_dependency(dep, "none", rocks_provided, verify, depskey)
         if not ok then
            return nil, err
         end
      end
      util.printout()
      return true
   elseif err then
      util.warning(err)
   end

   ok, err = check_supported_platforms(rockspec)
   if not ok then
      return nil, err
   end

   deps.report_missing_dependencies(name, version, rockspec[depskey], deps_mode, rocks_provided)

   util.printout()

   local get_versions = prepare_get_versions(deps_mode, rocks_provided, depskey)
   for _, dep in ipairs(rockspec[depskey]) do

      util.printout(("%s %s depends on %s (%s)"):format(
         name, version, tostring(dep), rock_status(dep, get_versions)))

      local ok, found_or_err, _, no_upgrade = deps.fulfill_dependency(dep, deps_mode, rocks_provided, verify, depskey)
      if ok then
         deplocks.add(depskey, dep.name, found_or_err)
      else
         if no_upgrade then
            util.printerr("This version of "..name.." is designed for use with")
            util.printerr(tostring(dep)..", but is configured to avoid upgrading it")
            util.printerr("automatically. Please upgrade "..dep.name.." with")
            util.printerr("   luarocks install "..dep.name)
            util.printerr("or look for a suitable version of "..name.." with")
            util.printerr("   luarocks search "..name)
         end
         return nil, found_or_err
      end
   end

   return true
end

--- If filename matches a pattern, return the capture.
-- For example, given "libfoo.so" and "lib?.so" is a pattern,
-- returns "foo" (which can then be used to build names
-- based on other patterns.
-- @param file string: a filename
-- @param pattern string: a pattern, where ? is to be matched by the filename.
-- @return string The pattern, if found, or nil.
local function deconstruct_pattern(file, pattern)
   local depattern = "^"..(pattern:gsub("%.", "%%."):gsub("%*", ".*"):gsub("?", "(.*)")).."$"
   return (file:match(depattern))
end

--- Construct all possible patterns for a name and add to the files array.
-- Run through the patterns array replacing all occurrences of "?"
-- with the given file name and store them in the files array.
-- @param file string A raw name (e.g. "foo")
-- @param array of string An array of patterns with "?" as the wildcard
-- (e.g. {"?.so", "lib?.so"})
-- @param files The array of constructed names
local function add_all_patterns(file, patterns, files)
   for _, pattern in ipairs(patterns) do
      table.insert(files, {#files + 1, (pattern:gsub("?", file))})
   end
end

local function get_external_deps_dirs(mode)
   local patterns = cfg.external_deps_patterns
   local subdirs = cfg.external_deps_subdirs
   if mode == "install" then
      patterns = cfg.runtime_external_deps_patterns
      subdirs = cfg.runtime_external_deps_subdirs
   end
   local dirs = {
      BINDIR = { subdir = subdirs.bin, testfile = "program", pattern = patterns.bin },
      INCDIR = { subdir = subdirs.include, testfile = "header", pattern = patterns.include },
      LIBDIR = { subdir = subdirs.lib, testfile = "library", pattern = patterns.lib }
   }
   if mode == "install" then
      dirs.INCDIR = nil
   end
   return dirs
end

local function resolve_prefix(prefix, dirs)
   if type(prefix) == "string" then
      return prefix
   elseif type(prefix) == "table" then
      if prefix.bin then
         dirs.BINDIR.subdir = prefix.bin
      end
      if prefix.include then
         if dirs.INCDIR then
            dirs.INCDIR.subdir = prefix.include
         end
      end
      if prefix.lib then
         dirs.LIBDIR.subdir = prefix.lib
      end
      return prefix.prefix
   end
end

local function add_patterns_for_file(files, file, patterns)
   -- If it doesn't look like it contains a filename extension
   if not (file:match("%.[a-z]+$") or file:match("%.[a-z]+%.")) then
      add_all_patterns(file, patterns, files)
   else
      for _, pattern in ipairs(patterns) do
         local matched = deconstruct_pattern(file, pattern)
         if matched then
            add_all_patterns(matched, patterns, files)
         end
      end
      table.insert(files, {#files + 1, file})
   end
end

local function check_external_dependency_at(prefix, name, ext_files, vars, dirs, err_files, cache)
   local fs = require("luarocks.fs")
   cache = cache or {}

   for dirname, dirdata in util.sortedpairs(dirs) do
      local paths
      local path_var_value = vars[name.."_"..dirname]
      if path_var_value then
         paths = { path_var_value }
      elseif type(dirdata.subdir) == "table" then
         paths = {}
         for i,v in ipairs(dirdata.subdir) do
            paths[i] = dir.path(prefix, v)
         end
      else
         paths = { dir.path(prefix, dirdata.subdir) }
      end
      local file_or_files = ext_files[dirdata.testfile]
      if file_or_files then
         local files = {}
         if type(file_or_files) == "string" then
            add_patterns_for_file(files, file_or_files, dirdata.pattern)
         elseif type(file_or_files) == "table" then
            for _, f in ipairs(file_or_files) do
               add_patterns_for_file(files, f, dirdata.pattern)
            end
         end

         local found = false
         table.sort(files, function(a, b)
            if (not a[2]:match("%*")) and b[2]:match("%*") then
               return true
            elseif a[2]:match("%*") and (not b[2]:match("%*")) then
               return false
            else
               return a[1] < b[1]
            end
         end)
         for _, fa in ipairs(files) do

            local f = fa[2]
            -- small convenience hack
            if f:match("%.so$") or f:match("%.dylib$") or f:match("%.dll$") then
               f = f:gsub("%.[^.]+$", "."..cfg.external_lib_extension)
            end

            local pattern
            if f:match("%*") then
               pattern = "^" .. f:gsub("([-.+])", "%%%1"):gsub("%*", ".*") .. "$"
               f = "matching "..f
            end

            for _, d in ipairs(paths) do
               if pattern then
                  if not cache[d] then
                     cache[d] = fs.list_dir(d)
                  end
                  local match = string.match
                  for _, entry in ipairs(cache[d]) do
                     if match(entry, pattern) then
                        found = true
                        break
                     end
                  end
               else
                  found = fs.is_file(dir.path(d, f))
               end
               if found then
                  dirdata.dir = d
                  dirdata.file = f
                  break
               else
                  table.insert(err_files[dirdata.testfile], f.." in "..d)
               end
            end
            if found then
               break
            end
         end
         if not found then
            return nil, dirname, dirdata.testfile
         end
      else
         -- When we have a set of subdir suffixes, look for one that exists.
         -- For these reason, we now put "lib" ahead of "" on Windows in our
         -- default set.
         dirdata.dir = paths[1]
         for _, p in ipairs(paths) do
            if fs.exists(p) then
               dirdata.dir = p
               break
            end
         end
      end
   end

   for dirname, dirdata in pairs(dirs) do
      vars[name.."_"..dirname] = dirdata.dir
      vars[name.."_"..dirname.."_FILE"] = dirdata.file
   end
   vars[name.."_DIR"] = prefix
   return true
end

local function check_external_dependency(name, ext_files, vars, mode, cache)
   local ok
   local err_dirname
   local err_testfile
   local err_files = {program = {}, header = {}, library = {}}

   local dirs = get_external_deps_dirs(mode)

   local prefixes
   if vars[name .. "_DIR"] then
      prefixes = { vars[name .. "_DIR"] }
   elseif vars.DEPS_DIR then
      prefixes = { vars.DEPS_DIR }
   else
      prefixes = cfg.external_deps_dirs
   end

   for _, prefix in ipairs(prefixes) do
      prefix = resolve_prefix(prefix, dirs)
      if cfg.is_platform("mingw32") and name == "LUA" then
         dirs.LIBDIR.pattern = fun.filter(util.deep_copy(dirs.LIBDIR.pattern), function(s)
            return not s:match("%.a$")
         end)
      elseif cfg.is_platform("windows") and name == "LUA" then
         dirs.LIBDIR.pattern = fun.filter(util.deep_copy(dirs.LIBDIR.pattern), function(s)
            return not s:match("%.dll$")
         end)
      end
      ok, err_dirname, err_testfile = check_external_dependency_at(prefix, name, ext_files, vars, dirs, err_files, cache)
      if ok then
         return true
      end
   end

   return nil, err_dirname, err_testfile, err_files
end

function deps.autodetect_external_dependencies(build)
   -- only applies to the 'builtin' build type
   if not build or not build.modules then
      return nil
   end

   local extdeps = {}
   local any = false
   for _, data in pairs(build.modules) do
      if type(data) == "table" and data.libraries then
         local libraries = data.libraries
         if type(libraries) == "string" then
            libraries = { libraries }
         end
         local incdirs = {}
         local libdirs = {}
         for _, lib in ipairs(libraries) do
            local upper = lib:upper():gsub("%+", "P"):gsub("[^%w]", "_")
            any = true
            extdeps[upper] = { library = lib }
            table.insert(incdirs, "$(" .. upper .. "_INCDIR)")
            table.insert(libdirs, "$(" .. upper .. "_LIBDIR)")
         end
         if not data.incdirs then
            data.incdirs = incdirs
         end
         if not data.libdirs then
            data.libdirs = libdirs
         end
      end
   end
   return any and extdeps or nil
end

--- Set up path-related variables for external dependencies.
-- For each key in the external_dependencies table in the
-- rockspec file, four variables are created: <key>_DIR, <key>_BINDIR,
-- <key>_INCDIR and <key>_LIBDIR. These are not overwritten
-- if already set (e.g. by the LuaRocks config file or through the
-- command-line). Values in the external_dependencies table
-- are tables that may contain a "header" or a "library" field,
-- with filenames to be tested for existence.
-- @param rockspec table: The rockspec table.
-- @param mode string: if "build" is given, checks all files;
-- if "install" is given, do not scan for headers.
-- @return boolean or (nil, string): True if no errors occurred, or
-- nil and an error message if any test failed.
function deps.check_external_deps(rockspec, mode)
   assert(rockspec:type() == "rockspec")

   if not rockspec.external_dependencies then
      rockspec.external_dependencies = deps.autodetect_external_dependencies(rockspec.build)
   end
   if not rockspec.external_dependencies then
      return true
   end

   for name, ext_files in util.sortedpairs(rockspec.external_dependencies) do
      local ok, err_dirname, err_testfile, err_files = check_external_dependency(name, ext_files, rockspec.variables, mode)
      if not ok then
         local lines = {"Could not find "..err_testfile.." file for "..name}

         local err_paths = {}
         for _, err_file in ipairs(err_files[err_testfile]) do
            if not err_paths[err_file] then
               err_paths[err_file] = true
               table.insert(lines, "  No file "..err_file)
            end
         end

         table.insert(lines, "You may have to install "..name.." in your system and/or pass "..name.."_DIR or "..name.."_"..err_dirname.." to the luarocks command.")
         table.insert(lines, "Example: luarocks install "..rockspec.name.." "..name.."_DIR=/usr/local")

         return nil, table.concat(lines, "\n"), "dependency"
      end
   end
   return true
end

--- Recursively add satisfied dependencies of a package to a table,
-- to build a transitive closure of all dependent packages.
-- Additionally ensures that `dependencies` table of the manifest is up-to-date.
-- @param results table: The results table being built, maps package names to versions.
-- @param mdeps table: The manifest dependencies table.
-- @param name string: Package name.
-- @param version string: Package version.
function deps.scan_deps(results, mdeps, name, version, deps_mode)
   assert(type(results) == "table")
   assert(type(mdeps) == "table")
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")

   local fetch = require("luarocks.fetch")

   if results[name] then
      return
   end
   if not mdeps[name] then mdeps[name] = {} end
   local mdn = mdeps[name]
   local dependencies = mdn[version]
   local rocks_provided
   if not dependencies then
      local rockspec, err = fetch.load_local_rockspec(path.rockspec_file(name, version), false)
      if not rockspec then
         return
      end
      dependencies = rockspec.dependencies
      rocks_provided = rockspec.rocks_provided
      mdn[version] = dependencies
   else
      rocks_provided = util.get_rocks_provided()
   end

   local get_versions = prepare_get_versions(deps_mode, rocks_provided, "dependencies")

   local matched = match_all_deps(dependencies, get_versions)
   results[name] = version
   for _, match in pairs(matched) do
      deps.scan_deps(results, mdeps, match.name, match.version, deps_mode)
   end
end

local function lua_h_exists(d, luaver)
   local major, minor = luaver:match("(%d+)%.(%d+)")
   local luanum = ("%s%02d"):format(major, tonumber(minor))

   local lua_h = dir.path(d, "lua.h")
   local fd = io.open(lua_h)
   if fd then
      local data = fd:read("*a")
      fd:close()
      if data:match("LUA_VERSION_NUM%s*" .. tostring(luanum)) then
         return d
      end
      return nil, "Lua header lua.h found at " .. d .. " does not match Lua version " .. luaver .. ". You can use `luarocks config variables.LUA_INCDIR <path>` to set the correct location.", "dependency", 2
   end

   return nil, "Failed finding Lua header lua.h (searched at " .. d .. "). You may need to install Lua development headers. You can use `luarocks config variables.LUA_INCDIR <path>` to set the correct location.", "dependency", 1
end

local function find_lua_incdir(prefix, luaver, luajitver)
   luajitver = luajitver and luajitver:gsub("%-.*", "")
   local shortv = luaver:gsub("%.", "")
   local incdirs = {
      prefix .. "/include/lua/" .. luaver,
      prefix .. "/include/lua" .. luaver,
      prefix .. "/include/lua-" .. luaver,
      prefix .. "/include/lua" .. shortv,
      prefix .. "/include",
      prefix,
      luajitver and (prefix .. "/include/luajit-" .. (luajitver:match("^(%d+%.%d+)") or "")),
   }
   local errprio = 0
   local mainerr
   for _, d in ipairs(incdirs) do
      local ok, err, _, prio = lua_h_exists(d, luaver)
      if ok then
         return d
      end
      if prio > errprio then
         mainerr = err
         errprio = prio
      end
   end

   -- not found, will fallback to a default
   return nil, mainerr
end

function deps.check_lua_incdir(vars)
   if vars.LUA_INCDIR_OK == true
      then return true
   end

   local ljv = util.get_luajit_version()

   if vars.LUA_INCDIR then
      local ok, err = lua_h_exists(vars.LUA_INCDIR, cfg.lua_version)
      if ok then
         vars.LUA_INCDIR_OK = true
      end
      return ok, err
   end

   if vars.LUA_DIR then
      local d, err = find_lua_incdir(vars.LUA_DIR, cfg.lua_version, ljv)
      if d then
         vars.LUA_INCDIR = d
         vars.LUA_INCDIR_OK = true
         return true
      end
      return nil, err
   end

   return nil, "Failed finding Lua headers; neither LUA_DIR or LUA_INCDIR are set. You may need to install them or configure LUA_INCDIR.", "dependency"
end

function deps.check_lua_libdir(vars)
   if vars.LUA_LIBDIR_OK == true
      then return true
   end

   local fs = require("luarocks.fs")
   local ljv = util.get_luajit_version()

   if vars.LUA_LIBDIR and vars.LUALIB and fs.exists(dir.path(vars.LUA_LIBDIR, vars.LUALIB)) then
      vars.LUA_LIBDIR_OK = true
      return true
   end

   local shortv = cfg.lua_version:gsub("%.", "")
   local libnames = {
      "lua" .. cfg.lua_version,
      "lua" .. shortv,
      "lua-" .. cfg.lua_version,
      "lua-" .. shortv,
      "lua",
   }
   if ljv then
      table.insert(libnames, 1, "luajit-" .. cfg.lua_version)
      table.insert(libnames, 2, "luajit")
   end
   local cache = {}
   local save_LUA_INCDIR = vars.LUA_INCDIR
   local ok, _, _, errfiles = check_external_dependency("LUA", { library = libnames }, vars, "build", cache)
   vars.LUA_INCDIR = save_LUA_INCDIR
   local err
   if ok then
      local filename = dir.path(vars.LUA_LIBDIR, vars.LUA_LIBDIR_FILE)
      local fd = io.open(filename, "r")
      if fd then
         if not vars.LUA_LIBDIR_FILE:match((cfg.lua_version:gsub("%.", "%%.?"))) then
            -- if filename isn't versioned, check file contents
            local txt = fd:read("*a")
            ok = txt:match("Lua " .. cfg.lua_version, 1, true)
                 or txt:match("lua" .. (cfg.lua_version:gsub("%.", "")), 1, true)
            if not ok then
               err = "Lua library at " .. filename .. " does not match Lua version " .. cfg.lua_version .. ". You can use `luarocks config variables.LUA_LIBDIR <path>` to set the correct location."
            end
         end

         fd:close()
      end
   end

   if ok then
      vars.LUALIB = vars.LUA_LIBDIR_FILE
      vars.LUA_LIBDIR_OK = true
      return true
   else
      err = err or "Failed finding the Lua library. You can use `luarocks config variables.LUA_LIBDIR <path>` to set the correct location."
      return nil, err, "dependency", errfiles
   end
end

function deps.get_deps_mode(args)
   return args.deps_mode or cfg.deps_mode
end

return deps
