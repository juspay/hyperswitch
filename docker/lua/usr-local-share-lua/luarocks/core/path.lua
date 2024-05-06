
--- Core LuaRocks-specific path handling functions.
local path = {}

local cfg = require("luarocks.core.cfg")
local dir = require("luarocks.core.dir")
local require = nil

local dir_sep = package.config:sub(1, 1)
--------------------------------------------------------------------------------

function path.rocks_dir(tree)
   if tree == nil then
      tree = cfg.root_dir
   end
   if type(tree) == "string" then
      return dir.path(tree, cfg.rocks_subdir)
   end
   assert(type(tree) == "table")
   return tree.rocks_dir or dir.path(tree.root, cfg.rocks_subdir)
end

--- Produce a versioned version of a filename.
-- @param file string: filename (must start with prefix)
-- @param prefix string: Path prefix for file
-- @param name string: Rock name
-- @param version string: Rock version
-- @return string: a pathname with the same directory parts and a versioned basename.
function path.versioned_name(file, prefix, name, version)
   assert(type(file) == "string")
   assert(type(name) == "string" and not name:match(dir_sep))
   assert(type(version) == "string")

   local rest = file:sub(#prefix+1):gsub("^" .. dir_sep .. "*", "")
   local name_version = (name.."_"..version):gsub("%-", "_"):gsub("%.", "_")
   return dir.path(prefix, name_version.."-"..rest)
end

--- Convert a pathname to a module identifier.
-- In Unix, for example, a path "foo/bar/baz.lua" is converted to
-- "foo.bar.baz"; "bla/init.lua" returns "bla.init"; "foo.so" returns "foo".
-- @param file string: Pathname of module
-- @return string: The module identifier, or nil if given path is
-- not a conformant module path (the function does not check if the
-- path actually exists).
function path.path_to_module(file)
   assert(type(file) == "string")

   local exts = {}
   local paths = package.path .. ";" .. package.cpath
   for entry in paths:gmatch("[^;]+") do
      local ext = entry:match("%.([a-z]+)$")
      if ext then
         exts[ext] = true
      end
   end

   local name
   for ext, _ in pairs(exts) do
      name = file:match("(.*)%." .. ext .. "$")
      if name then
         name = name:gsub("[\\/]", ".")
         break
      end
   end

   if not name then name = file end

   -- remove any beginning and trailing slashes-converted-to-dots
   name = name:gsub("^%.+", ""):gsub("%.+$", "")

   return name
end

function path.deploy_lua_dir(tree)
   if type(tree) == "string" then
      return dir.path(tree, cfg.lua_modules_path)
   else
      assert(type(tree) == "table")
      return tree.lua_dir or dir.path(tree.root, cfg.lua_modules_path)
   end
end

function path.deploy_lib_dir(tree)
   if type(tree) == "string" then
      return dir.path(tree, cfg.lib_modules_path)
   else
      assert(type(tree) == "table")
      return tree.lib_dir or dir.path(tree.root, cfg.lib_modules_path)
   end
end

local is_src_extension = { [".lua"] = true, [".tl"] = true, [".tld"] = true, [".moon"] = true }

--- Return the pathname of the file that would be loaded for a module, indexed.
-- @param file_name string: module file name as in manifest (eg. "socket/core.so")
-- @param name string: name of the package (eg. "luasocket")
-- @param version string: version number (eg. "2.0.2-1")
-- @param tree string: repository path (eg. "/usr/local")
-- @param i number: the index, 1 if version is the current default, > 1 otherwise.
-- This is done this way for use by select_module in luarocks.loader.
-- @return string: filename of the module (eg. "/usr/local/lib/lua/5.1/socket/core.so")
function path.which_i(file_name, name, version, tree, i)
   local deploy_dir
   local extension = file_name:match("%.[a-z]+$")
   if is_src_extension[extension] then
      deploy_dir = path.deploy_lua_dir(tree)
      file_name = dir.path(deploy_dir, file_name)
   else
      deploy_dir = path.deploy_lib_dir(tree)
      file_name = dir.path(deploy_dir, file_name)
   end
   if i > 1 then
      file_name = path.versioned_name(file_name, deploy_dir, name, version)
   end
   return file_name
end

function path.rocks_tree_to_string(tree)
   if type(tree) == "string" then
      return tree
   else
      assert(type(tree) == "table")
      return tree.root
   end
end

--- Apply a given function to the active rocks trees based on chosen dependency mode.
-- @param deps_mode string: Dependency mode: "one" for the current default tree,
-- "all" for all trees, "order" for all trees with priority >= the current default,
-- "none" for no trees (this function becomes a nop).
-- @param fn function: function to be applied, with the tree dir (string) as the first
-- argument and the remaining varargs of map_trees as the following arguments.
-- @return a table with all results of invocations of fn collected.
function path.map_trees(deps_mode, fn, ...)
   local result = {}
   local current = cfg.root_dir or cfg.rocks_trees[1]
   if deps_mode == "one" then
      table.insert(result, (fn(current, ...)) or 0)
   else
      local use = false
      if deps_mode == "all" then
         use = true
      end
      for _, tree in ipairs(cfg.rocks_trees or {}) do
         if dir.normalize(path.rocks_tree_to_string(tree)) == dir.normalize(path.rocks_tree_to_string(current)) then
            use = true
         end
         if use then
            table.insert(result, (fn(tree, ...)) or 0)
         end
      end
   end
   return result
end

return path
