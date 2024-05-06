
--- Module implementing the LuaRocks "doc" command.
-- Shows documentation for an installed rock.
local doc = {}

local util = require("luarocks.util")
local queries = require("luarocks.queries")
local search = require("luarocks.search")
local path = require("luarocks.path")
local dir = require("luarocks.dir")
local fetch = require("luarocks.fetch")
local fs = require("luarocks.fs")
local download = require("luarocks.download")

function doc.add_to_parser(parser)
   local cmd = parser:command("doc", "Show documentation for an installed rock.\n\n"..
      "Without any flags, tries to load the documentation using a series of heuristics.\n"..
      "With flags, return only the desired information.", util.see_also([[
   For more information about a rock, see the 'show' command.
]]))
      :summary("Show documentation for an installed rock.")

   cmd:argument("rock", "Name of the rock.")
      :action(util.namespaced_name_action)
   cmd:argument("version", "Version of the rock.")
      :args("?")

   cmd:flag("--home", "Open the home page of project.")
   cmd:flag("--list", "List documentation files only.")
   cmd:flag("--porcelain", "Produce machine-friendly output.")
end

local function show_homepage(homepage, name, namespace, version)
   if not homepage then
      return nil, "No 'homepage' field in rockspec for "..util.format_rock_name(name, namespace, version)
   end
   util.printout("Opening "..homepage.." ...")
   fs.browser(homepage)
   return true
end

local function try_to_open_homepage(name, namespace, version)
   local temp_dir, err = fs.make_temp_dir("doc-"..name.."-"..(version or ""))
   if not temp_dir then
      return nil, "Failed creating temporary directory: "..err
   end
   util.schedule_function(fs.delete, temp_dir)
   local ok, err = fs.change_dir(temp_dir)
   if not ok then return nil, err end
   local filename, err = download.download("rockspec", name, namespace, version)
   if not filename then return nil, err end
   local rockspec, err = fetch.load_local_rockspec(filename)
   if not rockspec then return nil, err end
   fs.pop_dir()
   local descript = rockspec.description or {}
   return show_homepage(descript.homepage, name, namespace, version)
end

--- Driver function for "doc" command.
-- @return boolean: True if succeeded, nil on errors.
function doc.command(args)
   local query = queries.new(args.rock, args.namespace, args.version)
   local iname, iversion, repo = search.pick_installed_rock(query, args.tree)
   if not iname then
      local rock = util.format_rock_name(args.rock, args.namespace, args.version)
      util.printout(rock.." is not installed. Looking for it in the rocks servers...")
      return try_to_open_homepage(args.rock, args.namespace, args.version)
   end
   local name, version = iname, iversion

   local rockspec, err = fetch.load_local_rockspec(path.rockspec_file(name, version, repo))
   if not rockspec then return nil,err end
   local descript = rockspec.description or {}

   if args.home then
      return show_homepage(descript.homepage, name, args.namespace, version)
   end

   local directory = path.install_dir(name, version, repo)

   local docdir
   local directories = { "doc", "docs" }
   for _, d in ipairs(directories) do
      local dirname = dir.path(directory, d)
      if fs.is_dir(dirname) then
         docdir = dirname
         break
      end
   end
   if not docdir then
      if descript.homepage and not args.list then
         util.printout("Local documentation directory not found -- opening "..descript.homepage.." ...")
         fs.browser(descript.homepage)
         return true
      end
      return nil, "Documentation directory not found for "..name.." "..version
   end

   docdir = dir.normalize(docdir)
   local files = fs.find(docdir)
   local htmlpatt = "%.html?$"
   local extensions = { htmlpatt, "%.md$", "%.txt$",  "%.textile$", "" }
   local basenames = { "index", "readme", "manual" }

   local porcelain = args.porcelain
   if #files > 0 then
      util.title("Documentation files for "..name.." "..version, porcelain)
      if porcelain then
         for _, file in ipairs(files) do
            util.printout(docdir.."/"..file)
         end
      else
         util.printout(docdir.."/")
         for _, file in ipairs(files) do
            util.printout("\t"..file)
         end
      end
   end

   if args.list then
      return true
   end

   for _, extension in ipairs(extensions) do
      for _, basename in ipairs(basenames) do
         local filename = basename..extension
         local found
         for _, file in ipairs(files) do
            if file:lower():match(filename) and ((not found) or #file < #found) then
               found = file
            end
         end
         if found then
            local pathname = dir.path(docdir, found)
            util.printout()
            util.printout("Opening "..pathname.." ...")
            util.printout()
            local ok = fs.browser(pathname)
            if not ok and not pathname:match(htmlpatt) then
               local fd = io.open(pathname, "r")
               util.printout(fd:read("*a"))
               fd:close()
            end
            return true
         end
      end
   end

   return true
end


return doc
