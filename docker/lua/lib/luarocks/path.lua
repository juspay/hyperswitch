
--- LuaRocks-specific path handling functions.
-- All paths are configured in this module, making it a single
-- point where the layout of the local installation is defined in LuaRocks.
local path = {}

local core = require("luarocks.core.path")
local dir = require("luarocks.dir")
local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")

path.rocks_dir = core.rocks_dir
path.versioned_name = core.versioned_name
path.path_to_module = core.path_to_module
path.deploy_lua_dir = core.deploy_lua_dir
path.deploy_lib_dir = core.deploy_lib_dir
path.map_trees = core.map_trees
path.rocks_tree_to_string = core.rocks_tree_to_string

--- Infer rockspec filename from a rock filename.
-- @param rock_name string: Pathname of a rock file.
-- @return string: Filename of the rockspec, without path.
function path.rockspec_name_from_rock(rock_name)
   assert(type(rock_name) == "string")
   local base_name = dir.base_name(rock_name)
   return base_name:match("(.*)%.[^.]*.rock") .. ".rockspec"
end

function path.root_from_rocks_dir(rocks_dir)
   assert(type(rocks_dir) == "string")
   return rocks_dir:match("(.*)" .. util.matchquote(cfg.rocks_subdir) .. ".*$")
end

function path.root_dir(tree)
   if type(tree) == "string" then
      return tree
   else
      assert(type(tree) == "table")
      return tree.root
   end
end

function path.deploy_bin_dir(tree)
   return dir.path(path.root_dir(tree), "bin")
end

function path.manifest_file(tree)
   return dir.path(path.rocks_dir(tree), "manifest")
end

--- Get the directory for all versions of a package in a tree.
-- @param name string: The package name.
-- @return string: The resulting path -- does not guarantee that
-- @param tree string or nil: If given, specifies the local tree to use.
-- the package (and by extension, the path) exists.
function path.versions_dir(name, tree)
   assert(type(name) == "string" and not name:match("/"))
   return dir.path(path.rocks_dir(tree), name)
end

--- Get the local installation directory (prefix) for a package.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the path) exists.
function path.install_dir(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version)
end

--- Get the local filename of the rockspec of an installed rock.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the file) exists.
function path.rockspec_file(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, name.."-"..version..".rockspec")
end

--- Get the local filename of the rock_manifest file of an installed rock.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the file) exists.
function path.rock_manifest_file(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "rock_manifest")
end

--- Get the local filename of the rock_namespace file of an installed rock.
-- @param name string: The package name (without a namespace).
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the file) exists.
function path.rock_namespace_file(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "rock_namespace")
end

--- Get the local installation directory for C libraries of a package.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the path) exists.
function path.lib_dir(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "lib")
end

--- Get the local installation directory for Lua modules of a package.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the path) exists.
function path.lua_dir(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "lua")
end

--- Get the local installation directory for documentation of a package.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the path) exists.
function path.doc_dir(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "doc")
end

--- Get the local installation directory for configuration files of a package.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the path) exists.
function path.conf_dir(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "conf")
end

--- Get the local installation directory for command-line scripts
-- of a package.
-- @param name string: The package name.
-- @param version string: The package version.
-- @param tree string or nil: If given, specifies the local tree to use.
-- @return string: The resulting path -- does not guarantee that
-- the package (and by extension, the path) exists.
function path.bin_dir(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   return dir.path(path.rocks_dir(tree), name, version, "bin")
end

--- Extract name, version and arch of a rock filename,
-- or name, version and "rockspec" from a rockspec name.
-- @param file_name string: pathname of a rock or rockspec
-- @return (string, string, string) or nil: name, version and arch
-- or nil if name could not be parsed
function path.parse_name(file_name)
   assert(type(file_name) == "string")
   if file_name:match("%.rock$") then
      return dir.base_name(file_name):match("(.*)-([^-]+-%d+)%.([^.]+)%.rock$")
   else
      return dir.base_name(file_name):match("(.*)-([^-]+-%d+)%.(rockspec)")
   end
end

--- Make a rockspec or rock URL.
-- @param pathname string: Base URL or pathname.
-- @param name string: Package name.
-- @param version string: Package version.
-- @param arch string: Architecture identifier, or "rockspec" or "installed".
-- @return string: A URL or pathname following LuaRocks naming conventions.
function path.make_url(pathname, name, version, arch)
   assert(type(pathname) == "string")
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   assert(type(arch) == "string")

   local filename = name.."-"..version
   if arch == "installed" then
      filename = dir.path(name, version, filename..".rockspec")
   elseif arch == "rockspec" then
      filename = filename..".rockspec"
   else
      filename = filename.."."..arch..".rock"
   end
   return dir.path(pathname, filename)
end

--- Obtain the directory name where a module should be stored.
-- For example, on Unix, "foo.bar.baz" will return "foo/bar".
-- @param mod string: A module name in Lua dot-separated format.
-- @return string: A directory name using the platform's separator.
function path.module_to_path(mod)
   assert(type(mod) == "string")
   return (mod:gsub("[^.]*$", ""):gsub("%.", "/"))
end

function path.use_tree(tree)
   cfg.root_dir = tree
   cfg.rocks_dir = path.rocks_dir(tree)
   cfg.deploy_bin_dir = path.deploy_bin_dir(tree)
   cfg.deploy_lua_dir = path.deploy_lua_dir(tree)
   cfg.deploy_lib_dir = path.deploy_lib_dir(tree)
end

function path.add_to_package_paths(tree)
   package.path = dir.path(path.deploy_lua_dir(tree), "?.lua") .. ";"
               .. dir.path(path.deploy_lua_dir(tree), "?/init.lua") .. ";"
               .. package.path
   package.cpath = dir.path(path.deploy_lib_dir(tree), "?." .. cfg.lib_extension) .. ";"
                .. package.cpath
end

--- Get the namespace of a locally-installed rock, if any.
-- @param name string: The rock name, without a namespace.
-- @param version string: The rock version.
-- @param tree string: The local tree to use.
-- @return string?: The namespace if it exists, or nil.
function path.read_namespace(name, version, tree)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")
   assert(type(tree) == "string")

   local namespace
   local fd = io.open(path.rock_namespace_file(name, version, tree), "r")
   if fd then
      namespace = fd:read("*a")
      fd:close()
   end
   return namespace
end

function path.package_paths(deps_mode)
   local lpaths = {}
   local lcpaths = {}
   path.map_trees(deps_mode, function(tree)
      local root = path.root_dir(tree)
      table.insert(lpaths, dir.path(root, cfg.lua_modules_path, "?.lua"))
      table.insert(lpaths, dir.path(root, cfg.lua_modules_path, "?/init.lua"))
      table.insert(lcpaths, dir.path(root, cfg.lib_modules_path, "?." .. cfg.lib_extension))
   end)
   return table.concat(lpaths, ";"), table.concat(lcpaths, ";")
end

return path
