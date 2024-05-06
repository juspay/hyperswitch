local remove = {}

local search = require("luarocks.search")
local deps = require("luarocks.deps")
local fetch = require("luarocks.fetch")
local repos = require("luarocks.repos")
local path = require("luarocks.path")
local util = require("luarocks.util")
local cfg = require("luarocks.core.cfg")
local manif = require("luarocks.manif")
local queries = require("luarocks.queries")

--- Obtain a list of packages that depend on the given set of packages
-- (where all packages of the set are versions of one program).
-- @param name string: the name of a program
-- @param versions array of string: the versions to be deleted.
-- @return array of string: an empty table if no packages depend on any
-- of the given list, or an array of strings in "name/version" format.
local function check_dependents(name, versions, deps_mode)
   local dependents = {}

   local skip_set = {}
   skip_set[name] = {}
   for version, _ in pairs(versions) do
      skip_set[name][version] = true
   end

   local local_rocks = {}
   local query_all = queries.all()
   search.local_manifest_search(local_rocks, cfg.rocks_dir, query_all)
   local_rocks[name] = nil
   for rock_name, rock_versions in pairs(local_rocks) do
      for rock_version, _ in pairs(rock_versions) do
         local rockspec, err = fetch.load_rockspec(path.rockspec_file(rock_name, rock_version))
         if rockspec then
            local _, missing = deps.match_deps(rockspec.dependencies, rockspec.rocks_provided, skip_set, deps_mode)
            if missing[name] then
               table.insert(dependents, { name = rock_name, version = rock_version })
            end
         end
      end
   end

   return dependents
end

--- Delete given versions of a program.
-- @param name string: the name of a program
-- @param versions array of string: the versions to be deleted.
-- @param deps_mode: string: Which trees to check dependencies for:
-- "one" for the current default tree, "all" for all trees,
-- "order" for all trees with priority >= the current default, "none" for no trees.
-- @return boolean or (nil, string): true on success or nil and an error message.
local function delete_versions(name, versions, deps_mode)

   for version, _ in pairs(versions) do
      util.printout("Removing "..name.." "..version.."...")
      local ok, err = repos.delete_version(name, version, deps_mode)
      if not ok then return nil, err end
   end

   return true
end

function remove.remove_search_results(results, name, deps_mode, force, fast)
   local versions = results[name]

   local version = next(versions)
   local second = next(versions, version)

   local dependents = {}
   if not fast then
      util.printout("Checking stability of dependencies in the absence of")
      util.printout(name.." "..table.concat(util.keys(versions), ", ").."...")
      util.printout()
      dependents = check_dependents(name, versions, deps_mode)
   end

   if #dependents > 0 then
      if force or fast then
         util.printerr("The following packages may be broken by this forced removal:")
         for _, dependent in ipairs(dependents) do
            util.printerr(dependent.name.." "..dependent.version)
         end
         util.printerr()
      else
         if not second then
            util.printerr("Will not remove "..name.." "..version..".")
            util.printerr("Removing it would break dependencies for: ")
         else
            util.printerr("Will not remove installed versions of "..name..".")
            util.printerr("Removing them would break dependencies for: ")
         end
         for _, dependent in ipairs(dependents) do
            util.printerr(dependent.name.." "..dependent.version)
         end
         util.printerr()
         util.printerr("Use --force to force removal (warning: this may break modules).")
         return nil, "Failed removing."
      end
   end

   local ok, err = delete_versions(name, versions, deps_mode)
   if not ok then return nil, err end

   util.printout("Removal successful.")
   return true
end

function remove.remove_other_versions(name, version, force, fast)
   local results = {}
   local query = queries.new(name, nil, version, false, nil, "~=")
   search.local_manifest_search(results, cfg.rocks_dir, query)
   local warn
   if results[name] then
      local ok, err = remove.remove_search_results(results, name, cfg.deps_mode, force, fast)
      if not ok then -- downgrade failure to a warning
         warn = err
      end
   end

   if not fast then
      -- since we're not using --keep, this means that all files of the rock being installed
      -- should be available as non-versioned variants. Double-check that:
      local rock_manifest, load_err = manif.load_rock_manifest(name, version)
      local ok, err = repos.check_everything_is_installed(name, version, rock_manifest, cfg.root_dir, false)
      if not ok then
         return nil, err
      end
   end

   return true, nil, warn
end

return remove
