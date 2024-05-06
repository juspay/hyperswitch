--- Module for handling manifest files and tables.
-- Manifest files describe the contents of a LuaRocks tree or server.
-- They are loaded into manifest tables, which are then used for
-- performing searches, matching dependencies, etc.
local manif = {}

local core = require("luarocks.core.manif")
local persist = require("luarocks.persist")
local fetch = require("luarocks.fetch")
local dir = require("luarocks.dir")
local fs = require("luarocks.fs")
local cfg = require("luarocks.core.cfg")
local path = require("luarocks.path")
local util = require("luarocks.util")
local queries = require("luarocks.queries")
local type_manifest = require("luarocks.type.manifest")

manif.cache_manifest = core.cache_manifest
manif.load_rocks_tree_manifests = core.load_rocks_tree_manifests
manif.scan_dependencies = core.scan_dependencies

manif.rock_manifest_cache = {}

local function check_manifest(repo_url, manifest, globals)
   local ok, err = type_manifest.check(manifest, globals)
   if not ok then
      core.cache_manifest(repo_url, cfg.lua_version, nil)
      return nil, "Error checking manifest: "..err, "type"
   end
   return manifest
end

local postprocess_dependencies
do
   local postprocess_check = setmetatable({}, { __mode = "k" })
   postprocess_dependencies = function(manifest)
      if postprocess_check[manifest] then
         return
      end
      if manifest.dependencies then
         for name, versions in pairs(manifest.dependencies) do
            for version, entries in pairs(versions) do
               for k, v in pairs(entries) do
                  entries[k] = queries.from_persisted_table(v)
               end
            end
         end
      end
      postprocess_check[manifest] = true
   end
end

function manif.load_rock_manifest(name, version, root)
   assert(type(name) == "string" and not name:match("/"))
   assert(type(version) == "string")

   local name_version = name.."/"..version
   if manif.rock_manifest_cache[name_version] then
      return manif.rock_manifest_cache[name_version].rock_manifest
   end
   local pathname = path.rock_manifest_file(name, version, root)
   local rock_manifest = persist.load_into_table(pathname)
   if not rock_manifest then
      return nil, "rock_manifest file not found for "..name.." "..version.." - not a LuaRocks tree?"
   end
   manif.rock_manifest_cache[name_version] = rock_manifest
   return rock_manifest.rock_manifest
end

--- Load a local or remote manifest describing a repository.
-- All functions that use manifest tables assume they were obtained
-- through this function.
-- @param repo_url string: URL or pathname for the repository.
-- @param lua_version string: Lua version in "5.x" format, defaults to installed version.
-- @param versioned_only boolean: If true, do not fall back to the main manifest
-- if a versioned manifest was not found.
-- @return table or (nil, string, [string]): A table representing the manifest,
-- or nil followed by an error message and an optional error code.
function manif.load_manifest(repo_url, lua_version, versioned_only)
   assert(type(repo_url) == "string")
   assert(type(lua_version) == "string" or not lua_version)
   lua_version = lua_version or cfg.lua_version

   local cached_manifest = core.get_cached_manifest(repo_url, lua_version)
   if cached_manifest then
      postprocess_dependencies(cached_manifest)
      return cached_manifest
   end

   local filenames = {
      "manifest-"..lua_version..".zip",
      "manifest-"..lua_version,
      not versioned_only and "manifest" or nil,
   }

   local protocol, repodir = dir.split_url(repo_url)
   local pathname, from_cache
   if protocol == "file" then
      for _, filename in ipairs(filenames) do
         pathname = dir.path(repodir, filename)
         if fs.exists(pathname) then
            break
         end
      end
   else
      local err, errcode
      for _, filename in ipairs(filenames) do
         pathname, err, errcode, from_cache = fetch.fetch_caching(dir.path(repo_url, filename), "no_mirror")
         if pathname then
            break
         end
      end
      if not pathname then
         return nil, err, errcode
      end
   end
   if pathname:match(".*%.zip$") then
      pathname = fs.absolute_name(pathname)
      local nozip = pathname:match("(.*)%.zip$")
      if not from_cache then
         local dirname = dir.dir_name(pathname)
         fs.change_dir(dirname)
         fs.delete(nozip)
         local ok, err = fs.unzip(pathname)
         fs.pop_dir()
         if not ok then
            fs.delete(pathname)
            fs.delete(pathname..".timestamp")
            return nil, "Failed extracting manifest file: " .. err
         end
      end
      pathname = nozip
   end
   local manifest, err, errcode = core.manifest_loader(pathname, repo_url, lua_version)
   if not manifest then
      return nil, err, errcode
   end

   postprocess_dependencies(manifest)
   return check_manifest(repo_url, manifest, err)
end

--- Get type and name of an item (a module or a command) provided by a file.
-- @param deploy_type string: rock manifest subtree the file comes from ("bin", "lua", or "lib").
-- @param file_path string: path to the file relatively to deploy_type subdirectory.
-- @return (string, string): item type ("module" or "command") and name.
function manif.get_provided_item(deploy_type, file_path)
   assert(type(deploy_type) == "string")
   assert(type(file_path) == "string")
   local item_type = deploy_type == "bin" and "command" or "module"
   local item_name = item_type == "command" and file_path or path.path_to_module(file_path)
   return item_type, item_name
end

local function get_providers(item_type, item_name, repo)
   assert(type(item_type) == "string")
   assert(type(item_name) == "string")
   local rocks_dir = path.rocks_dir(repo or cfg.root_dir)
   local manifest = manif.load_manifest(rocks_dir)
   return manifest and manifest[item_type .. "s"][item_name]
end

--- Given a name of a module or a command, figure out which rock name and version
-- correspond to it in the rock tree manifest.
-- @param item_type string: "module" or "command".
-- @param item_name string: module or command name.
-- @param root string or nil: A local root dir for a rocks tree. If not given, the default is used.
-- @return (string, string) or nil: name and version of the provider rock or nil if there
-- is no provider.
function manif.get_current_provider(item_type, item_name, repo)
   local providers = get_providers(item_type, item_name, repo)
   if providers then
      return providers[1]:match("([^/]*)/([^/]*)")
   end
end

function manif.get_next_provider(item_type, item_name, repo)
   local providers = get_providers(item_type, item_name, repo)
   if providers and providers[2] then
      return providers[2]:match("([^/]*)/([^/]*)")
   end
end

--- Get all versions of a package listed in a manifest file.
-- @param name string: a package name.
-- @param deps_mode string: "one", to use only the currently
-- configured tree; "order" to select trees based on order
-- (use the current tree and all trees below it on the list)
-- or "all", to use all trees.
-- @return table: An array of strings listing installed
-- versions of a package, and a table indicating where they are found.
function manif.get_versions(dep, deps_mode)
   assert(type(dep) == "table")
   assert(type(deps_mode) == "string")

   local name = dep.name
   local namespace = dep.namespace

   local version_set = {}
   path.map_trees(deps_mode, function(tree)
      local manifest = manif.load_manifest(path.rocks_dir(tree))

      if manifest and manifest.repository[name] then
         for version in pairs(manifest.repository[name]) do
            if dep.namespace then
               local ns_file = path.rock_namespace_file(name, version, tree)
               local fd = io.open(ns_file, "r")
               if fd then
                  local ns = fd:read("*a")
                  fd:close()
                  if ns == namespace then
                     version_set[version] = tree
                  end
               end
            else
               version_set[version] = tree
            end
         end
      end
   end)

   return util.keys(version_set), version_set
end

return manif
