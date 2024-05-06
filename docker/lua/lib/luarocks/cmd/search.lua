
--- Module implementing the LuaRocks "search" command.
-- Queries LuaRocks servers.
local cmd_search = {}

local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")
local search = require("luarocks.search")
local queries = require("luarocks.queries")
local results = require("luarocks.results")

function cmd_search.add_to_parser(parser)
   local cmd = parser:command("search", "Query the LuaRocks servers.", util.see_also())

   cmd:argument("name", "Name of the rock to search for.")
      :args("?")
      :action(util.namespaced_name_action)
   cmd:argument("version", "Rock version to search for.")
      :args("?")

   cmd:flag("--source", "Return only rockspecs and source rocks, to be used "..
      'with the "build" command.')
   cmd:flag("--binary", "Return only pure Lua and binary rocks (rocks that "..
      'can be used with the "install" command without requiring a C toolchain).')
   cmd:flag("--all", "List all contents of the server that are suitable to "..
      "this platform, do not filter by name.")
   cmd:flag("--porcelain", "Return a machine readable format.")
end

--- Splits a list of search results into two lists, one for "source" results
-- to be used with the "build" command, and one for "binary" results to be
-- used with the "install" command.
-- @param result_tree table: A search results table.
-- @return (table, table): Two tables, one for source and one for binary
-- results.
local function split_source_and_binary_results(result_tree)
   local sources, binaries = {}, {}
   for name, versions in pairs(result_tree) do
      for version, repositories in pairs(versions) do
         for _, repo in ipairs(repositories) do
            local where = sources
            if repo.arch == "all" or repo.arch == cfg.arch then
               where = binaries
            end
            local entry = results.new(name, version, repo.repo, repo.arch)
            search.store_result(where, entry)
         end
      end
   end
   return sources, binaries
end

--- Driver function for "search" command.
-- @return boolean or (nil, string): True if build was successful; nil and an
-- error message otherwise.
function cmd_search.command(args)
   local name = args.name

   if args.all then
      name, args.version = "", nil
   end

   if not args.name and not args.all then
      return nil, "Enter name and version or use --all. "..util.see_help("search")
   end

   local query = queries.new(name, args.namespace, args.version, true)
   local result_tree, err = search.search_repos(query)
   local porcelain = args.porcelain
   local full_name = util.format_rock_name(name, args.namespace, args.version)
   util.title(full_name .. " - Search results for Lua "..cfg.lua_version..":", porcelain, "=")
   local sources, binaries = split_source_and_binary_results(result_tree)
   if next(sources) and not args.binary then
      util.title("Rockspecs and source rocks:", porcelain)
      search.print_result_tree(sources, porcelain)
   end
   if next(binaries) and not args.source then
      util.title("Binary and pure-Lua rocks:", porcelain)
      search.print_result_tree(binaries, porcelain)
   end
   return true
end

return cmd_search
