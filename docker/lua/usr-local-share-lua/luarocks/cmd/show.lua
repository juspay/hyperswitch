--- Module implementing the LuaRocks "show" command.
-- Shows information about an installed rock.
local show = {}

local queries = require("luarocks.queries")
local search = require("luarocks.search")
local dir = require("luarocks.core.dir")
local fs = require("luarocks.fs")
local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")
local path = require("luarocks.path")
local fetch = require("luarocks.fetch")
local manif = require("luarocks.manif")
local repos = require("luarocks.repos")

function show.add_to_parser(parser)
   local cmd = parser:command("show", [[
Show information about an installed rock.

Without any flags, show all module information.
With flags, return only the desired information.]], util.see_also())
      :summary("Show information about an installed rock.")

   cmd:argument("rock", "Name of an installed rock.")
      :action(util.namespaced_name_action)
   cmd:argument("version", "Rock version.")
      :args("?")

   cmd:flag("--home", "Show home page of project.")
   cmd:flag("--modules", "Show all modules provided by the package as used by require().")
   cmd:flag("--deps", "Show packages the package depends on.")
   cmd:flag("--build-deps", "Show build-only dependencies for the package.")
   cmd:flag("--test-deps", "Show dependencies for testing the package.")
   cmd:flag("--rockspec", "Show the full path of the rockspec file.")
   cmd:flag("--mversion", "Show the package version.")
   cmd:flag("--rock-tree", "Show local tree where rock is installed.")
   cmd:flag("--rock-namespace", "Show rock namespace.")
   cmd:flag("--rock-dir", "Show data directory of the installed rock.")
   cmd:flag("--rock-license", "Show rock license.")
   cmd:flag("--issues", "Show URL for project's issue tracker.")
   cmd:flag("--labels", "List the labels of the rock.")
   cmd:flag("--porcelain", "Produce machine-friendly output.")
end

local friendly_template = [[
          :
?namespace:${namespace}/${package} ${version} - ${summary}
!namespace:${package} ${version} - ${summary}
          :
*detailed :${detailed}
?detailed :
?license  :License:      \t${license}
?homepage :Homepage:     \t${homepage}
?issues   :Issues:       \t${issues}
?labels   :Labels:       \t${labels}
?location :Installed in: \t${location}
?commands :
?commands :Commands:
*commands :\t${name} (${file})
?modules  :
?modules  :Modules:
*modules  :\t${name} (${file})
?bdeps    :
?bdeps    :Has build dependency on:
*bdeps    :\t${name} (${label})
?tdeps    :
?tdeps    :Tests depend on:
*tdeps    :\t${name} (${label})
?deps     :
?deps     :Depends on:
*deps     :\t${name} (${label})
?ideps    :
?ideps    :Indirectly pulling:
*ideps    :\t${name} (${label})
          :
]]

local porcelain_template = [[
?namespace:namespace\t${namespace}
?package  :package\t${package}
?version  :version\t${version}
?summary  :summary\t${summary}
*detailed :detailed\t${detailed}
?license  :license\t${license}
?homepage :homepage\t${homepage}
?issues   :issues\t${issues}
?labels   :labels\t${labels}
?location :location\t${location}
*commands :command\t${name}\t${file}
*modules  :module\t${name}\t${file}
*bdeps    :build_dependency\t${name}\t${label}
*tdeps    :test_dependency\t${name}\t${label}
*deps     :dependency\t${name}\t${label}
*ideps    :indirect_dependency\t${name}\t${label}
]]

local function keys_as_string(t, sep)
   local keys = util.keys(t)
   table.sort(keys)
   return table.concat(keys, sep or " ")
end

local function word_wrap(line)
   local width = tonumber(os.getenv("COLUMNS")) or 80
   if width > 80 then width = 80 end
   if #line > width then
      local brk = width
      while brk > 0 and line:sub(brk, brk) ~= " " do
         brk = brk - 1
      end
      if brk > 0 then
         return line:sub(1, brk-1) .. "\n" .. word_wrap(line:sub(brk+1))
      end
   end
   return line
end

local function format_text(text)
   text = text:gsub("^%s*",""):gsub("%s$", ""):gsub("\n[ \t]+","\n"):gsub("([^\n])\n([^\n])","%1 %2")
   local paragraphs = util.split_string(text, "\n\n")
   for n, line in ipairs(paragraphs) do
      paragraphs[n] = word_wrap(line)
   end
   return (table.concat(paragraphs, "\n\n"):gsub("%s$", ""))
end

local function installed_rock_label(dep, tree)
   local installed, version
   local rocks_provided = util.get_rocks_provided()
   if rocks_provided[dep.name] then
      installed, version = true, rocks_provided[dep.name]
   else
      installed, version = search.pick_installed_rock(dep, tree)
   end
   return installed and "using "..version or "missing"
end

local function render(template, data)
   local out = {}
   for cmd, var, line in template:gmatch("(.)([a-z]*)%s*:([^\n]*)\n") do
      line = line:gsub("\\t", "\t")
      local d = data[var]
      if cmd == " " then
         table.insert(out, line)
      elseif cmd == "?" or cmd == "*" or cmd == "!" then
         if (cmd == "!" and d == nil)
             or (cmd ~= "!" and (type(d) == "string"
                                 or (type(d) == "table" and next(d)))) then
            local n = cmd == "*" and #d or 1
            for i = 1, n do
               local tbl = cmd == "*" and d[i] or data
               if type(tbl) == "string" then
                  tbl = tbl:gsub("%%", "%%%%")
               end
               table.insert(out, (line:gsub("${([a-z]+)}", tbl)))
            end
         end
      end
   end
   return table.concat(out, "\n")
end

local function adjust_path(name, version, basedir, pathname, suffix)
   pathname = dir.path(basedir, pathname)
   local vpathname = path.versioned_name(pathname, basedir, name, version)
   return (fs.exists(vpathname)
          and vpathname
          or pathname) .. (suffix or "")
end

local function modules_to_list(name, version, repo)
   local ret = {}
   local rock_manifest = manif.load_rock_manifest(name, version, repo)

   local lua_dir = path.deploy_lua_dir(repo)
   local lib_dir = path.deploy_lib_dir(repo)
   repos.recurse_rock_manifest_entry(rock_manifest.lua, function(pathname)
      table.insert(ret, {
         name = path.path_to_module(pathname),
         file = adjust_path(name, version, lua_dir, pathname),
      })
   end)
   repos.recurse_rock_manifest_entry(rock_manifest.lib, function(pathname)
      table.insert(ret, {
         name = path.path_to_module(pathname),
         file = adjust_path(name, version, lib_dir, pathname),
      })
   end)
   table.sort(ret, function(a, b)
      if a.name == b.name then
         return a.file < b.file
      end
      return a.name < b.name
   end)
   return ret
end

local function commands_to_list(name, version, repo)
   local ret = {}
   local rock_manifest = manif.load_rock_manifest(name, version, repo)

   local bin_dir = path.deploy_bin_dir(repo)
   repos.recurse_rock_manifest_entry(rock_manifest.bin, function(pathname)
      table.insert(ret, {
         name = name,
         file = adjust_path(name, version, bin_dir, pathname, cfg.wrapper_suffix),
      })
   end)
   table.sort(ret, function(a, b)
      if a.name == b.name then
         return a.file < b.file
      end
      return a.name < b.name
   end)
   return ret
end

local function deps_to_list(dependencies, tree)
   local ret = {}
   for _, dep in ipairs(dependencies or {}) do
      table.insert(ret, { name = tostring(dep), label = installed_rock_label(dep, tree) })
   end
   return ret
end

local function indirect_deps(mdeps, rdeps, tree)
   local ret = {}
   local direct_deps = {}
   for _, dep in ipairs(rdeps) do
      direct_deps[dep] = true
   end
   for dep_name in util.sortedpairs(mdeps or {}) do
      if not direct_deps[dep_name] then
         table.insert(ret, { name = tostring(dep_name), label = installed_rock_label(queries.new(dep_name), tree) })
      end
   end
   return ret
end

local function show_rock(template, namespace, name, version, rockspec, repo, minfo, tree)
   local desc = rockspec.description or {}
   local data = {
      namespace = namespace,
      package = rockspec.package,
      version = rockspec.version,
      summary = desc.summary or "",
      detailed = desc.detailed and util.split_string(format_text(desc.detailed), "\n"),
      license = desc.license,
      homepage = desc.homepage,
      issues = desc.issues_url,
      labels = desc.labels and table.concat(desc.labels, ", "),
      location = path.rocks_tree_to_string(repo),
      commands = commands_to_list(name, version, repo),
      modules = modules_to_list(name, version, repo),
      bdeps = deps_to_list(rockspec.build_dependencies, tree),
      tdeps = deps_to_list(rockspec.test_dependencies, tree),
      deps = deps_to_list(rockspec.dependencies, tree),
      ideps = indirect_deps(minfo.dependencies, rockspec.dependencies, tree),
   }
   util.printout(render(template, data))
end

--- Driver function for "show" command.
-- @return boolean: True if succeeded, nil on errors.
function show.command(args)
   local query = queries.new(args.rock, args.namespace, args.version, true)

   local name, version, repo, repo_url = search.pick_installed_rock(query, args.tree)
   if not name then
      return nil, version
   end
   local tree = path.rocks_tree_to_string(repo)
   local directory = path.install_dir(name, version, repo)
   local namespace = path.read_namespace(name, version, tree)
   local rockspec_file = path.rockspec_file(name, version, repo)
   local rockspec, err = fetch.load_local_rockspec(rockspec_file)
   if not rockspec then return nil,err end

   local descript = rockspec.description or {}
   local manifest, err = manif.load_manifest(repo_url)
   if not manifest then return nil,err end
   local minfo = manifest.repository[name][version][1]

   if args.rock_tree then util.printout(tree)
   elseif args.rock_namespace then util.printout(namespace)
   elseif args.rock_dir then util.printout(directory)
   elseif args.home then util.printout(descript.homepage)
   elseif args.rock_license then util.printout(descript.license)
   elseif args.issues then util.printout(descript.issues_url)
   elseif args.labels then util.printout(descript.labels and table.concat(descript.labels, "\n"))
   elseif args.modules then util.printout(keys_as_string(minfo.modules, "\n"))
   elseif args.deps then
      for _, dep in ipairs(rockspec.dependencies) do
         util.printout(tostring(dep))
      end
   elseif args.build_deps then
      for _, dep in ipairs(rockspec.build_dependencies) do
         util.printout(tostring(dep))
      end
   elseif args.test_deps then
      for _, dep in ipairs(rockspec.test_dependencies) do
         util.printout(tostring(dep))
      end
   elseif args.rockspec then util.printout(rockspec_file)
   elseif args.mversion then util.printout(version)
   elseif args.porcelain then
      show_rock(porcelain_template, namespace, name, version, rockspec, repo, minfo, args.tree)
   else
      show_rock(friendly_template, namespace, name, version, rockspec, repo, minfo, args.tree)
   end
   return true
end

return show
