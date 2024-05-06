
--- Module implementing the luarocks-admin "make_manifest" command.
-- Compile a manifest file for a repository.
local make_manifest = {}

local writer = require("luarocks.manif.writer")
local index = require("luarocks.admin.index")
local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")
local deps = require("luarocks.deps")
local fs = require("luarocks.fs")
local dir = require("luarocks.dir")

function make_manifest.add_to_parser(parser)
   local cmd = parser:command("make_manifest", "Compile a manifest file for a repository.", util.see_also())

   cmd:argument("repository", "Local repository pathname.")
      :args("?")

   cmd:flag("--local-tree", "If given, do not write versioned versions of the manifest file.\n"..
      "Use this when rebuilding the manifest of a local rocks tree.")
   util.deps_mode_option(cmd)
end

--- Driver function for "make_manifest" command.
-- @return boolean or (nil, string): True if manifest was generated,
-- or nil and an error message.
function make_manifest.command(args)
   local repo = args.repository or cfg.rocks_dir

   util.printout("Making manifest for "..repo)

   if repo:match("/lib/luarocks") and not args.local_tree then
      util.warning("This looks like a local rocks tree, but you did not pass --local-tree.")
   end

   local ok, err = writer.make_manifest(repo, deps.get_deps_mode(args), not args.local_tree)
   if ok and not args.local_tree then
      util.printout("Generating index.html for "..repo)
      index.make_index(repo)
   end
   if args.local_tree then
      for luaver in util.lua_versions() do
         fs.delete(dir.path(repo, "manifest-"..luaver))
      end
   end
   return ok, err
end

return make_manifest
