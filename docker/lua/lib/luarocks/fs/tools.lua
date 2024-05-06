
--- Common fs operations implemented with third-party tools.
local tools = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local cfg = require("luarocks.core.cfg")

local vars = setmetatable({}, { __index = function(_,k) return cfg.variables[k] end })

local dir_stack = {}

do
   local tool_cache = {}

   local tool_options = {
      downloader = {
         desc = "downloader",
         { var = "WGET", name = "wget" },
         { var = "CURL", name = "curl" },
      },
      md5checker = {
         desc = "MD5 checker",
         { var = "MD5SUM", name = "md5sum" },
         { var = "OPENSSL", name = "openssl", cmdarg = "md5" },
         { var = "MD5", name = "md5" },
      },
   }

   function tools.which_tool(tooltype)
      local tool = tool_cache[tooltype]
      local names = {}
      if not tool then
         for _, opt in ipairs(tool_options[tooltype]) do
            table.insert(names, opt.name)
            if fs.is_tool_available(vars[opt.var], opt.name) then
               tool = opt
               tool_cache[tooltype] = opt
               break
            end
         end
      end
      if not tool then
         local tool_names = table.concat(names, ", ", 1, #names - 1) .. " or " .. names[#names]
         return nil, "no " .. tool_options[tooltype].desc .. " tool available," .. " please install " .. tool_names .. " in your system"
      end
      return tool.name, vars[tool.var] .. (tool.cmdarg and " "..tool.cmdarg or "")
   end
end

local current_dir_with_cache
do
   local cache_pwd

   current_dir_with_cache = function()
      local current = cache_pwd
      if not current then
         local pipe = io.popen(fs.quiet_stderr(vars.PWD))
         current = pipe:read("*a"):gsub("^%s*", ""):gsub("%s*$", "")
         pipe:close()
         cache_pwd = current
      end
      for _, directory in ipairs(dir_stack) do
         current = fs.absolute_name(directory, current)
      end
      return current, cache_pwd
   end

   --- Obtain current directory.
   -- Uses the module's internal directory stack.
   -- @return string: the absolute pathname of the current directory.
   function tools.current_dir()
      return (current_dir_with_cache()) -- drop second return
   end
end

--- Change the current directory.
-- Uses the module's internal directory stack. This does not have exact
-- semantics of chdir, as it does not handle errors the same way,
-- but works well for our purposes for now.
-- @param directory string: The directory to switch to.
-- @return boolean or (nil, string): true if successful, (nil, error message) if failed.
function tools.change_dir(directory)
   assert(type(directory) == "string")
   if fs.is_dir(directory) then
      table.insert(dir_stack, directory)
      return true
   end
   return nil, "directory not found: "..directory
end

--- Change directory to root.
-- Allows leaving a directory (e.g. for deleting it) in
-- a crossplatform way.
function tools.change_dir_to_root()
   local curr_dir = fs.current_dir()
   if not curr_dir or not fs.is_dir(curr_dir) then
      return false
   end
   table.insert(dir_stack, "/")
   return true
end

--- Change working directory to the previous in the directory stack.
function tools.pop_dir()
   local directory = table.remove(dir_stack)
   return directory ~= nil
end

--- Run the given command.
-- The command is executed in the current directory in the directory stack.
-- @param cmd string: No quoting/escaping is applied to the command.
-- @return boolean: true if command succeeds (status code 0), false
-- otherwise.
function tools.execute_string(cmd)
   local current, cache_pwd = current_dir_with_cache()
   if not current then return false end
   if current ~= cache_pwd then
      cmd = fs.command_at(current, cmd)
   end
   local code = os.execute(cmd)
   if code == 0 or code == true then
      return true
   else
      return false
   end
end

--- Internal implementation function for fs.dir.
-- Yields a filename on each iteration.
-- @param at string: directory to list
-- @return nil
function tools.dir_iterator(at)
   local pipe = io.popen(fs.command_at(at, vars.LS, true))
   for file in pipe:lines() do
      if file ~= "." and file ~= ".." then
         coroutine.yield(file)
      end
   end
   pipe:close()
end

--- Download a remote file.
-- @param url string: URL to be fetched.
-- @param filename string or nil: this function attempts to detect the
-- resulting local filename of the remote file as the basename of the URL;
-- if that is not correct (due to a redirection, for example), the local
-- filename can be given explicitly as this second argument.
-- @param cache boolean: compare remote timestamps via HTTP HEAD prior to
-- re-downloading the file.
-- @return (boolean, string, string): true and the filename on success,
-- false and the error message and code on failure.
function tools.use_downloader(url, filename, cache)
   assert(type(url) == "string")
   assert(type(filename) == "string" or not filename)

   filename = fs.absolute_name(filename or dir.base_name(url))

   local downloader, err = fs.which_tool("downloader")
   if not downloader then
      return nil, err, "downloader"
   end

   local ok = false
   if downloader == "wget" then
      local wget_cmd = vars.WGET.." "..vars.WGETNOCERTFLAG.." --no-cache --user-agent=\""..cfg.user_agent.." via wget\" --quiet "
      if cfg.connection_timeout and cfg.connection_timeout > 0 then
        wget_cmd = wget_cmd .. "--timeout="..tostring(cfg.connection_timeout).." --tries=1 "
      end
      if cache then
         -- --timestamping is incompatible with --output-document,
         -- but that's not a problem for our use cases.
         fs.delete(filename .. ".unixtime")
         fs.change_dir(dir.dir_name(filename))
         ok = fs.execute_quiet(wget_cmd.." --timestamping ", url)
         fs.pop_dir()
      elseif filename then
         ok = fs.execute_quiet(wget_cmd.." --output-document ", filename, url)
      else
         ok = fs.execute_quiet(wget_cmd, url)
      end
   elseif downloader == "curl" then
      local curl_cmd = vars.CURL.." "..vars.CURLNOCERTFLAG.." -f -L --user-agent \""..cfg.user_agent.." via curl\" "
      if cfg.connection_timeout and cfg.connection_timeout > 0 then
        curl_cmd = curl_cmd .. "--connect-timeout "..tostring(cfg.connection_timeout).." "
      end
      if cache then
         curl_cmd = curl_cmd .. " -R -z \"" .. filename .. "\" "
      end
      ok = fs.execute_string(fs.quiet_stderr(curl_cmd..fs.Q(url).." --output "..fs.Q(filename)))
   end
   if ok then
      return true, filename
   else
      os.remove(filename)
      return false, "failed downloading " .. url, "network"
   end
end

--- Get the MD5 checksum for a file.
-- @param file string: The file to be computed.
-- @return string: The MD5 checksum or nil + message
function tools.get_md5(file)
   local ok, md5checker = fs.which_tool("md5checker")
   if not ok then
      return false, md5checker
   end

   local pipe = io.popen(md5checker.." "..fs.Q(fs.absolute_name(file)))
   local computed = pipe:read("*l")
   pipe:close()
   if computed then
      computed = computed:match("("..("%x"):rep(32)..")")
   end
   if computed then
      return computed
   else
      return nil, "Failed to compute MD5 hash for file "..tostring(fs.absolute_name(file))
   end
end

return tools
