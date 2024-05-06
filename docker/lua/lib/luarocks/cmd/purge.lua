
--- Module implementing the LuaRocks "purge" command.
-- Remove all rocks from a given tree.
local purge = {}

local util = require("luarocks.util")
local path = require("luarocks.path")
local search = require("luarocks.search")
local vers = require("luarocks.core.vers")
local repos = require("luarocks.repos")
local writer = require("luarocks.manif.writer")
local cfg = require("luarocks.core.cfg")
local remove = require("luarocks.remove")
local queries = require("luarocks.queries")

function purge.add_to_parser(parser)
   -- luacheck: push ignore 431
   local cmd = parser:command("purge", [[
This command removes rocks en masse from a given tree.
By default, it removes all rocks from a tree.

The --tree option is mandatory: luarocks purge does not assume a default tree.]],
   util.see_also())
      :summary("Remove all installed rocks from a tree.")
   -- luacheck: pop

   cmd:flag("--old-versions", "Keep the highest-numbered version of each "..
      "rock and remove the other ones. By default it only removes old "..
      "versions if they are not needed as dependencies. This can be "..
      "overridden with the flag --force.")
   cmd:flag("--force", "If --old-versions is specified, force removal of "..
      "previously installed versions if it would break dependencies.")
   cmd:flag("--force-fast", "Like --force, but performs a forced removal "..
      "without reporting dependency issues.")
end

function purge.command(args)
   local tree = args.tree

   local results = {}
   search.local_manifest_search(results, path.rocks_dir(tree), queries.all())

   local sort = function(a,b) return vers.compare_versions(b,a) end
   if args.old_versions then
      sort = vers.compare_versions
   end

   for package, versions in util.sortedpairs(results) do
      for version, _ in util.sortedpairs(versions, sort) do
         if args.old_versions then
            util.printout("Keeping "..package.." "..version.."...")
            local ok, err, warn = remove.remove_other_versions(package, version, args.force, args.force_fast)
            if not ok then
               util.printerr(err)
            elseif warn then
               util.printerr(err)
            end
            break
         else
            util.printout("Removing "..package.." "..version.."...")
            local ok, err = repos.delete_version(package, version, "none", true)
            if not ok then
               util.printerr(err)
            end
         end
      end
   end
   return writer.make_manifest(cfg.rocks_dir, "one")
end

purge.needs_lock = function() return true end

return purge
