--- Module implementing the LuaRocks "install" command.
-- Installs binary rocks.
local install = {}

local path = require("luarocks.path")
local repos = require("luarocks.repos")
local fetch = require("luarocks.fetch")
local util = require("luarocks.util")
local fs = require("luarocks.fs")
local deps = require("luarocks.deps")
local writer = require("luarocks.manif.writer")
local remove = require("luarocks.remove")
local search = require("luarocks.search")
local queries = require("luarocks.queries")
local cfg = require("luarocks.core.cfg")

function install.add_to_parser(parser)
   local cmd = parser:command("install", "Install a rock.", util.see_also())  -- luacheck: ignore 431

   cmd:argument("rock", "The name of a rock to be fetched from a repository "..
      "or a filename of a locally available rock.")
      :action(util.namespaced_name_action)
   cmd:argument("version", "Version of the rock.")
      :args("?")

   cmd:flag("--keep", "Do not remove previously installed versions of the "..
      "rock after building a new one. This behavior can be made permanent by "..
      "setting keep_other_versions=true in the configuration file.")
   cmd:flag("--force", "If --keep is not specified, force removal of "..
      "previously installed versions if it would break dependencies. "..
      "If rock is already installed, reinstall it anyway.")
   cmd:flag("--force-fast", "Like --force, but performs a forced removal "..
      "without reporting dependency issues.")
   cmd:flag("--only-deps --deps-only", "Install only the dependencies of the rock.")
   cmd:flag("--no-doc", "Install the rock without its documentation.")
   cmd:flag("--verify", "Verify signature of the rockspec or src.rock being "..
      "built. If the rockspec or src.rock is being downloaded, LuaRocks will "..
      "attempt to download the signature as well. Otherwise, the signature "..
      "file should be already available locally in the same directory.\n"..
      "You need the signer’s public key in your local keyring for this "..
      "option to work properly.")
   cmd:flag("--check-lua-versions", "If the rock can't be found, check repository "..
      "and report if it is available for another Lua version.")
   util.deps_mode_option(cmd)
   cmd:flag("--no-manifest", "Skip creating/updating the manifest")
   cmd:flag("--pin", "If the installed rock is a Lua module, create a "..
      "luarocks.lock file listing the exact versions of each dependency found for "..
      "this rock (recursively), and store it in the rock's directory. "..
      "Ignores any existing luarocks.lock file in the rock's sources.")
   -- luarocks build options
   parser:flag("--pack-binary-rock"):hidden(true)
   parser:option("--branch"):hidden(true)
   parser:flag("--sign"):hidden(true)
end

install.opts = util.opts_table("install.opts", {
   namespace = "string?",
   keep = "boolean",
   force = "boolean",
   force_fast = "boolean",
   no_doc = "boolean",
   deps_mode = "string",
   verify = "boolean",
})

--- Install a binary rock.
-- @param rock_file string: local or remote filename of a rock.
-- @param opts table: installation options
-- @return (string, string) or (nil, string, [string]): Name and version of
-- installed rock if succeeded or nil and an error message followed by an error code.
function install.install_binary_rock(rock_file, opts)
   assert(type(rock_file) == "string")
   assert(opts:type() == "install.opts")

   local namespace = opts.namespace
   local deps_mode = opts.deps_mode

   local name, version, arch = path.parse_name(rock_file)
   if not name then
      return nil, "Filename "..rock_file.." does not match format 'name-version-revision.arch.rock'."
   end

   if arch ~= "all" and arch ~= cfg.arch then
      return nil, "Incompatible architecture "..arch, "arch"
   end
   if repos.is_installed(name, version) then
      if (not opts.force) and (not opts.force_fast) then
         util.printout(name .. " " .. version .. " is already installed in " .. path.root_dir(cfg.root_dir))
         util.printout("Use --force to reinstall.")
         return name, version
      end
      repos.delete_version(name, version, opts.deps_mode)
   end

   local install_dir = path.install_dir(name, version)

   local rollback = util.schedule_function(function()
      fs.delete(install_dir)
      fs.remove_dir_if_empty(path.versions_dir(name))
   end)

   local ok, err, errcode = fetch.fetch_and_unpack_rock(rock_file, install_dir, opts.verify)
   if not ok then return nil, err, errcode end

   local rockspec, err = fetch.load_rockspec(path.rockspec_file(name, version))
   if err then
      return nil, "Failed loading rockspec for installed package: "..err, errcode
   end

   if opts.deps_mode ~= "none" then
      ok, err, errcode = deps.check_external_deps(rockspec, "install")
      if err then return nil, err, errcode end
   end

   -- For compatibility with .rock files built with LuaRocks 1
   if not fs.exists(path.rock_manifest_file(name, version)) then
      ok, err = writer.make_rock_manifest(name, version)
      if err then return nil, err end
   end

   if namespace then
      ok, err = writer.make_namespace_file(name, version, namespace)
      if err then return nil, err end
   end

   if deps_mode ~= "none" then
      ok, err, errcode = deps.fulfill_dependencies(rockspec, "dependencies", deps_mode, opts.verify, install_dir)
      if err then return nil, err, errcode end
   end

   ok, err = repos.deploy_files(name, version, repos.should_wrap_bin_scripts(rockspec), deps_mode)
   if err then return nil, err end

   util.remove_scheduled_function(rollback)
   rollback = util.schedule_function(function()
      repos.delete_version(name, version, deps_mode)
   end)

   ok, err = repos.run_hook(rockspec, "post_install")
   if err then return nil, err end

   util.announce_install(rockspec)
   util.remove_scheduled_function(rollback)
   return name, version
end

--- Installs the dependencies of a binary rock.
-- @param rock_file string: local or remote filename of a rock.
-- @param opts table: installation options
-- @return (string, string) or (nil, string, [string]): Name and version of
-- the rock whose dependencies were installed if succeeded or nil and an error message
-- followed by an error code.
function install.install_binary_rock_deps(rock_file, opts)
   assert(type(rock_file) == "string")
   assert(opts:type() == "install.opts")

   local name, version, arch = path.parse_name(rock_file)
   if not name then
      return nil, "Filename "..rock_file.." does not match format 'name-version-revision.arch.rock'."
   end

   if arch ~= "all" and arch ~= cfg.arch then
      return nil, "Incompatible architecture "..arch, "arch"
   end

   local install_dir = path.install_dir(name, version)

   local ok, err, errcode = fetch.fetch_and_unpack_rock(rock_file, install_dir, opts.verify)
   if not ok then return nil, err, errcode end

   local rockspec, err = fetch.load_rockspec(path.rockspec_file(name, version))
   if err then
      return nil, "Failed loading rockspec for installed package: "..err, errcode
   end

   ok, err, errcode = deps.fulfill_dependencies(rockspec, "dependencies", opts.deps_mode, opts.verify, install_dir)
   if err then return nil, err, errcode end

   util.printout()
   util.printout("Successfully installed dependencies for " ..name.." "..version)

   return name, version
end

local function install_rock_file_deps(filename, opts)
   assert(opts:type() == "install.opts")

   local name, version = install.install_binary_rock_deps(filename, opts)
   if not name then return nil, version end

   writer.check_dependencies(nil, opts.deps_mode)
   return name, version
end

local function install_rock_file(filename, opts)
   assert(type(filename) == "string")
   assert(opts:type() == "install.opts")

   local name, version = install.install_binary_rock(filename, opts)
   if not name then return nil, version end

   if opts.no_doc then
      util.remove_doc_dir(name, version)
   end

   if (not opts.keep) and not cfg.keep_other_versions then
      local ok, err, warn = remove.remove_other_versions(name, version, opts.force, opts.force_fast)
      if not ok then
         return nil, err
      elseif warn then
         util.printerr(err)
      end
   end

   writer.check_dependencies(nil, opts.deps_mode)
   return name, version
end

--- Driver function for the "install" command.
-- If an URL or pathname to a binary rock is given, fetches and installs it.
-- If a rockspec or a source rock is given, forwards the request to the "build"
-- command.
-- If a package name is given, forwards the request to "search" and,
-- if returned a result, installs the matching rock.
-- @return boolean or (nil, string, exitcode): True if installation was
-- successful, nil and an error message otherwise. exitcode is optionally returned.
function install.command(args)
   if args.rock:match("%.rockspec$") or args.rock:match("%.src%.rock$") then
      local build = require("luarocks.cmd.build")
      return build.command(args)
   elseif args.rock:match("%.rock$") then
      local deps_mode = deps.get_deps_mode(args)
      local opts = install.opts({
         namespace = args.namespace,
         keep = not not args.keep,
         force = not not args.force,
         force_fast = not not args.force_fast,
         no_doc = not not args.no_doc,
         deps_mode = deps_mode,
         verify = not not args.verify,
      })
      if args.only_deps then
         return install_rock_file_deps(args.rock, opts)
      else
         return install_rock_file(args.rock, opts)
      end
   else
      local url, err = search.find_rock_checking_lua_versions(
                          queries.new(args.rock, args.namespace, args.version),
                          args.check_lua_versions)
      if not url then
         return nil, err
      end
      util.printout("Installing "..url)
      args.rock = url
      return install.command(args)
   end
end

install.needs_lock = function(args)
   if args.pack_binary_rock then
      return false
   end
   return true
end

return install
