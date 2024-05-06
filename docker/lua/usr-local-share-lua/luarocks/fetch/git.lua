
--- Fetch back-end for retrieving sources from GIT.
local git = {}

local unpack = unpack or table.unpack

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local vers = require("luarocks.core.vers")
local util = require("luarocks.util")

local cached_git_version

--- Get git version.
-- @param git_cmd string: name of git command.
-- @return table: git version as returned by luarocks.core.vers.parse_version.
local function git_version(git_cmd)
   if not cached_git_version then
      local version_line = io.popen(fs.Q(git_cmd)..' --version'):read()
      local version_string = version_line:match('%d-%.%d+%.?%d*')
      cached_git_version = vers.parse_version(version_string)
   end

   return cached_git_version
end

--- Check if git satisfies version requirement.
-- @param git_cmd string: name of git command.
-- @param version string: required version.
-- @return boolean: true if git matches version or is newer, false otherwise.
local function git_is_at_least(git_cmd, version)
   return git_version(git_cmd) >= vers.parse_version(version)
end

--- Git >= 1.7.10 can clone a branch **or tag**, < 1.7.10 by branch only. We
-- need to know this in order to build the appropriate command; if we can't
-- clone by tag then we'll have to issue a subsequent command to check out the
-- given tag.
-- @param git_cmd string: name of git command.
-- @return boolean: Whether Git can clone by tag.
local function git_can_clone_by_tag(git_cmd)
   return git_is_at_least(git_cmd, "1.7.10")
end

--- Git >= 1.8.4 can fetch submodules shallowly, saving bandwidth and time for
-- submodules with large history.
-- @param git_cmd string: name of git command.
-- @return boolean: Whether Git can fetch submodules shallowly.
local function git_supports_shallow_submodules(git_cmd)
   return git_is_at_least(git_cmd, "1.8.4")
end

--- Git >= 2.10 can fetch submodules shallowly according to .gitmodules configuration, allowing more granularity.
-- @param git_cmd string: name of git command.
-- @return boolean: Whether Git can fetch submodules shallowly according to .gitmodules.
local function git_supports_shallow_recommendations(git_cmd)
   return git_is_at_least(git_cmd, "2.10.0")
end

local function git_identifier(git_cmd, ver)
   if not (ver:match("^dev%-%d+$") or ver:match("^scm%-%d+$")) then
      return nil
   end
   local pd = io.popen(fs.command_at(fs.current_dir(), fs.Q(git_cmd).." log --pretty=format:%ai_%h -n 1"))
   if not pd then
      return nil
   end
   local date_hash = pd:read("*l")
   pd:close()
   if not date_hash then
      return nil
   end
   local date, time, tz, hash = date_hash:match("([^%s]+) ([^%s]+) ([^%s]+)_([^%s]+)")
   date = date:gsub("%-", "")
   time = time:gsub(":", "")
   return date .. "." .. time .. "." .. hash
end

--- Download sources for building a rock, using git.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Unused in this module (required for API purposes.)
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function git.get_sources(rockspec, extract, dest_dir, depth)
   assert(rockspec:type() == "rockspec")
   assert(type(dest_dir) == "string" or not dest_dir)

   local git_cmd = rockspec.variables.GIT
   local name_version = rockspec.name .. "-" .. rockspec.version
   local module = dir.base_name(rockspec.source.url)
   -- Strip off .git from base name if present
   module = module:gsub("%.git$", "")

   local ok, err_msg = fs.is_tool_available(git_cmd, "Git")
   if not ok then
      return nil, err_msg
   end

   local store_dir
   if not dest_dir then
      store_dir = fs.make_temp_dir(name_version)
      if not store_dir then
         return nil, "Failed creating temporary directory."
      end
      util.schedule_function(fs.delete, store_dir)
   else
      store_dir = dest_dir
   end
   store_dir = fs.absolute_name(store_dir)
   local ok, err = fs.change_dir(store_dir)
   if not ok then return nil, err end

   local command = {fs.Q(git_cmd), "clone", depth or "--depth=1", rockspec.source.url, module}
   local tag_or_branch = rockspec.source.tag or rockspec.source.branch
   -- If the tag or branch is explicitly set to "master" in the rockspec, then
   -- we can avoid passing it to Git since it's the default.
   if tag_or_branch == "master" then tag_or_branch = nil end
   if tag_or_branch then
      if git_can_clone_by_tag(git_cmd) then
         -- The argument to `--branch` can actually be a branch or a tag as of
         -- Git 1.7.10.
         table.insert(command, 3, "--branch=" .. tag_or_branch)
      end
   end
   if not fs.execute(unpack(command)) then
      return nil, "Failed cloning git repository."
   end
   ok, err = fs.change_dir(module)
   if not ok then return nil, err end
   if tag_or_branch and not git_can_clone_by_tag() then
      if not fs.execute(fs.Q(git_cmd), "checkout", tag_or_branch) then
         return nil, 'Failed to check out the "' .. tag_or_branch ..'" tag or branch.'
      end
   end

   -- Fetching git submodules is supported only when rockspec format is >= 3.0.
   if rockspec:format_is_at_least("3.0") then
      command = {fs.Q(git_cmd), "submodule", "update", "--init", "--recursive"}

      if git_supports_shallow_recommendations(git_cmd) then
         table.insert(command, 5, "--recommend-shallow")
      elseif git_supports_shallow_submodules(git_cmd) then
         -- Fetch only the last commit of each submodule.
         table.insert(command, 5, "--depth=1")
      end

      if not fs.execute(unpack(command)) then
         return nil, 'Failed to fetch submodules.'
      end
   end

   if not rockspec.source.tag then
      rockspec.source.identifier = git_identifier(git_cmd, rockspec.version)
   end

   fs.delete(dir.path(store_dir, module, ".git"))
   fs.delete(dir.path(store_dir, module, ".gitignore"))
   fs.pop_dir()
   fs.pop_dir()
   return module, store_dir
end

return git
