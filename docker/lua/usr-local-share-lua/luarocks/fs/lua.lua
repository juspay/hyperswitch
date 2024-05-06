
--- Native Lua implementation of filesystem and platform abstractions,
-- using LuaFileSystem, LuaSocket, LuaSec, lua-zlib, LuaPosix, MD5.
-- module("luarocks.fs.lua")
local fs_lua = {}

local fs = require("luarocks.fs")

local cfg = require("luarocks.core.cfg")
local dir = require("luarocks.dir")
local util = require("luarocks.util")
local vers = require("luarocks.core.vers")

local pack = table.pack or function(...) return { n = select("#", ...), ... } end

local socket_ok, zip_ok, lfs_ok, md5_ok, posix_ok, bz2_ok, _
local http, ftp, zip, lfs, md5, posix, bz2

if cfg.fs_use_modules then
   socket_ok, http = pcall(require, "socket.http")
   _, ftp = pcall(require, "socket.ftp")
   zip_ok, zip = pcall(require, "luarocks.tools.zip")
   bz2_ok, bz2 = pcall(require, "bz2")
   lfs_ok, lfs = pcall(require, "lfs")
   md5_ok, md5 = pcall(require, "md5")
   posix_ok, posix = pcall(require, "posix")
end

local patch = require("luarocks.tools.patch")
local tar = require("luarocks.tools.tar")

local dir_sep = package.config:sub(1, 1)

local dir_stack = {}

--- Test is file/dir is writable.
-- Warning: testing if a file/dir is writable does not guarantee
-- that it will remain writable and therefore it is no replacement
-- for checking the result of subsequent operations.
-- @param file string: filename to test
-- @return boolean: true if file exists, false otherwise.
function fs_lua.is_writable(file)
   assert(file)
   file = dir.normalize(file)
   local result
   if fs.is_dir(file) then
      local file2 = dir.path(file, '.tmpluarockstestwritable')
      local fh = io.open(file2, 'wb')
      result = fh ~= nil
      if fh then fh:close() end
      os.remove(file2)
   else
      local fh = io.open(file, 'r+b')
      result = fh ~= nil
      if fh then fh:close() end
   end
   return result
end

function fs_lua.quote_args(command, ...)
   local out = { command }
   local args = pack(...)
   for i=1, args.n do
      local arg = args[i]
      assert(type(arg) == "string")
      out[#out+1] = fs.Q(arg)
   end
   return table.concat(out, " ")
end

--- Run the given command, quoting its arguments.
-- The command is executed in the current directory in the dir stack.
-- @param command string: The command to be executed. No quoting/escaping
-- is applied.
-- @param ... Strings containing additional arguments, which are quoted.
-- @return boolean: true if command succeeds (status code 0), false
-- otherwise.
function fs_lua.execute(command, ...)
   assert(type(command) == "string")
   return fs.execute_string(fs.quote_args(command, ...))
end

--- Run the given command, quoting its arguments, silencing its output.
-- The command is executed in the current directory in the dir stack.
-- Silencing is omitted if 'verbose' mode is enabled.
-- @param command string: The command to be executed. No quoting/escaping
-- is applied.
-- @param ... Strings containing additional arguments, which will be quoted.
-- @return boolean: true if command succeeds (status code 0), false
-- otherwise.
function fs_lua.execute_quiet(command, ...)
   assert(type(command) == "string")
   if cfg.verbose then -- omit silencing output
      return fs.execute_string(fs.quote_args(command, ...))
   else
      return fs.execute_string(fs.quiet(fs.quote_args(command, ...)))
   end
end

function fs_lua.execute_env(env, command, ...)
   assert(type(command) == "string")
   local envstr = {}
   for var, val in pairs(env) do
      table.insert(envstr, fs.export_cmd(var, val))
   end
   return fs.execute_string(table.concat(envstr, "\n") .. "\n" .. fs.quote_args(command, ...))
end

local tool_available_cache = {}

function fs_lua.set_tool_available(tool_name, value)
   assert(type(value) == "boolean")
   tool_available_cache[tool_name] = value
end

--- Checks if the given tool is available.
-- The tool is executed using a flag, usually just to ask its version.
-- @param tool_cmd string: The command to be used to check the tool's presence (e.g. hg in case of Mercurial)
-- @param tool_name string: The actual name of the tool (e.g. Mercurial)
function fs_lua.is_tool_available(tool_cmd, tool_name)
   assert(type(tool_cmd) == "string")
   assert(type(tool_name) == "string")

   local ok
   if tool_available_cache[tool_name] ~= nil then
      ok = tool_available_cache[tool_name]
   else
      local tool_cmd_no_args = tool_cmd:gsub(" [^\"]*$", "")

      -- if it looks like the tool has a pathname, try that first
      if tool_cmd_no_args:match("[/\\]") then
         local tool_cmd_no_args_normalized = dir.path(tool_cmd_no_args)
         local fd = io.open(tool_cmd_no_args_normalized, "r")
         if fd then
            fd:close()
            ok = true
         end
      end

      if not ok then
         ok = fs.search_in_path(tool_cmd_no_args)
      end

      tool_available_cache[tool_name] = (ok == true)
   end

   if ok then
      return true
   else
      local msg = "'%s' program not found. Make sure %s is installed and is available in your PATH " ..
                  "(or you may want to edit the 'variables.%s' value in file '%s')"
      return nil, msg:format(tool_cmd, tool_name, tool_name:upper(), cfg.config_files.nearest)
   end
end

--- Check the MD5 checksum for a file.
-- @param file string: The file to be checked.
-- @param md5sum string: The string with the expected MD5 checksum.
-- @return boolean: true if the MD5 checksum for 'file' equals 'md5sum', false + msg if not
-- or if it could not perform the check for any reason.
function fs_lua.check_md5(file, md5sum)
   file = dir.normalize(file)
   local computed, msg = fs.get_md5(file)
   if not computed then
      return false, msg
   end
   if computed:match("^"..md5sum) then
      return true
   else
      return false, "Mismatch MD5 hash for file "..file
   end
end

--- List the contents of a directory.
-- @param at string or nil: directory to list (will be the current
-- directory if none is given).
-- @return table: an array of strings with the filenames representing
-- the contents of a directory.
function fs_lua.list_dir(at)
   local result = {}
   for file in fs.dir(at) do
      result[#result+1] = file
   end
   return result
end

--- Iterate over the contents of a directory.
-- @param at string or nil: directory to list (will be the current
-- directory if none is given).
-- @return function: an iterator function suitable for use with
-- the for statement.
function fs_lua.dir(at)
   if not at then
      at = fs.current_dir()
   end
   at = dir.normalize(at)
   if not fs.is_dir(at) then
      return function() end
   end
   return coroutine.wrap(function() fs.dir_iterator(at) end)
end

--- List the Lua modules at a specific require path.
-- eg. `modules("luarocks.cmd")` would return a list of all LuaRocks command
-- modules, in the current Lua path.
function fs_lua.modules(at)
   at = at or ""
   if #at > 0 then
      -- turn require path into file path
      at = at:gsub("%.", package.config:sub(1,1)) .. package.config:sub(1,1)
   end

   local path = package.path:sub(-1, -1) == ";" and package.path or package.path .. ";"
   local paths = {}
   for location in path:gmatch("(.-);") do
      if location:lower() == "?.lua" then
         location = "./?.lua"
      end
      local _, q_count = location:gsub("%?", "") -- only use the ones with a single '?'
      if location:match("%?%.[lL][uU][aA]$") and q_count == 1 then  -- only use when ending with "?.lua"
         location = location:gsub("%?%.[lL][uU][aA]$", at)
         table.insert(paths, location)
      end
   end

   if #paths == 0 then
      return {}
   end

   local modules = {}
   local is_duplicate = {}
   for _, path in ipairs(paths) do  -- luacheck: ignore 421
      local files = fs.list_dir(path)
      for _, filename in ipairs(files or {}) do
         if filename:match("%.[lL][uU][aA]$") then
           filename = filename:sub(1,-5) -- drop the extension
           if not is_duplicate[filename] then
              is_duplicate[filename] = true
              table.insert(modules, filename)
           end
         end
      end
   end

   return modules
end

function fs_lua.filter_file(fn, input_filename, output_filename)
   local fd, err = io.open(input_filename, "rb")
   if not fd then
      return nil, err
   end

   local input, err = fd:read("*a")
   fd:close()
   if not input then
      return nil, err
   end

   local output, err = fn(input)
   if not output then
      return nil, err
   end

   fd, err = io.open(output_filename, "wb")
   if not fd then
      return nil, err
   end

   local ok, err = fd:write(output)
   fd:close()
   if not ok then
      return nil, err
   end

   return true
end

function fs_lua.system_temp_dir()
   return os.getenv("TMPDIR") or os.getenv("TEMP") or "/tmp"
end

local function temp_dir_pattern(name_pattern)
   return dir.path(fs.system_temp_dir(),
                   "luarocks_" .. dir.normalize(name_pattern):gsub("[/\\]", "_") .. "-")
end

---------------------------------------------------------------------
-- LuaFileSystem functions
---------------------------------------------------------------------

if lfs_ok then

function fs_lua.file_age(filename)
   local attr = lfs.attributes(filename)
   if attr and attr.change then
      return os.difftime(os.time(), attr.change)
   end
   return math.huge
end

function fs_lua.lock_access(dirname, force)
   fs.make_dir(dirname)
   local lockfile = dir.path(dirname, "lockfile.lfs")

   -- drop stale lock, older than 1 hour
   local age = fs.file_age(lockfile)
   if age > 3600 and age < math.huge then
      force = true
   end

   if force then
      os.remove(lockfile)
   end
   return lfs.lock_dir(dirname)
end

function fs_lua.unlock_access(lock)
   return lock:free()
end

--- Run the given command.
-- The command is executed in the current directory in the dir stack.
-- @param cmd string: No quoting/escaping is applied to the command.
-- @return boolean: true if command succeeds (status code 0), false
-- otherwise.
function fs_lua.execute_string(cmd)
   local code = os.execute(cmd)
   return (code == 0 or code == true)
end

--- Obtain current directory.
-- Uses the module's internal dir stack.
-- @return string: the absolute pathname of the current directory.
function fs_lua.current_dir()
   return lfs.currentdir()
end

--- Change the current directory.
-- Uses the module's internal dir stack. This does not have exact
-- semantics of chdir, as it does not handle errors the same way,
-- but works well for our purposes for now.
-- @param d string: The directory to switch to.
function fs_lua.change_dir(d)
   table.insert(dir_stack, lfs.currentdir())
   d = dir.normalize(d)
   return lfs.chdir(d)
end

--- Change directory to root.
-- Allows leaving a directory (e.g. for deleting it) in
-- a crossplatform way.
function fs_lua.change_dir_to_root()
   local current = lfs.currentdir()
   if not current or current == "" then
      return false
   end
   table.insert(dir_stack, current)
   lfs.chdir("/") -- works on Windows too
   return true
end

--- Change working directory to the previous in the dir stack.
-- @return true if a pop occurred, false if the stack was empty.
function fs_lua.pop_dir()
   local d = table.remove(dir_stack)
   if d then
      lfs.chdir(d)
      return true
   else
      return false
   end
end

--- Create a directory if it does not already exist.
-- If any of the higher levels in the path name do not exist
-- too, they are created as well.
-- @param directory string: pathname of directory to create.
-- @return boolean or (boolean, string): true on success or (false, error message) on failure.
function fs_lua.make_dir(directory)
   assert(type(directory) == "string")
   directory = dir.normalize(directory)
   local path = nil
   if directory:sub(2, 2) == ":" then
     path = directory:sub(1, 2)
     directory = directory:sub(4)
   else
     if directory:match("^" .. dir_sep) then
        path = ""
     end
   end
   for d in directory:gmatch("([^" .. dir_sep .. "]+)" .. dir_sep .. "*") do
      path = path and path .. dir_sep .. d or d
      local mode = lfs.attributes(path, "mode")
      if not mode then
         local ok, err = lfs.mkdir(path)
         if not ok then
            return false, err
         end
         if cfg.is_platform("unix") then
            ok, err = fs.set_permissions(path, "exec", "all")
            if not ok then
               return false, err
            end
         end
      elseif mode ~= "directory" then
         return false, path.." is not a directory"
      end
   end
   return true
end

--- Remove a directory if it is empty.
-- Does not return errors (for example, if directory is not empty or
-- if already does not exist)
-- @param d string: pathname of directory to remove.
function fs_lua.remove_dir_if_empty(d)
   assert(d)
   d = dir.normalize(d)
   lfs.rmdir(d)
end

--- Remove a directory if it is empty.
-- Does not return errors (for example, if directory is not empty or
-- if already does not exist)
-- @param d string: pathname of directory to remove.
function fs_lua.remove_dir_tree_if_empty(d)
   assert(d)
   d = dir.normalize(d)
   for i=1,10 do
      lfs.rmdir(d)
      d = dir.dir_name(d)
   end
end

local function are_the_same_file(f1, f2)
   if f1 == f2 then
      return true
   end
   if cfg.is_platform("unix") then
      local i1 = lfs.attributes(f1, "ino")
      local i2 = lfs.attributes(f2, "ino")
      if i1 ~= nil and i1 == i2 then
         return true
      end
   end
   return false
end

--- Copy a file.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @param perms string ("read" or "exec") or nil: Permissions for destination
-- file or nil to use the source file permissions
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function fs_lua.copy(src, dest, perms)
   assert(src and dest)
   src = dir.normalize(src)
   dest = dir.normalize(dest)
   local destmode = lfs.attributes(dest, "mode")
   if destmode == "directory" then
      dest = dir.path(dest, dir.base_name(src))
   end
   if are_the_same_file(src, dest) then
      return nil, "The source and destination are the same files"
   end
   local src_h, err = io.open(src, "rb")
   if not src_h then return nil, err end
   local dest_h, err = io.open(dest, "w+b")
   if not dest_h then src_h:close() return nil, err end
   while true do
      local block = src_h:read(8192)
      if not block then break end
      local ok, err = dest_h:write(block)
      if not ok then return nil, err end
   end
   src_h:close()
   dest_h:close()

   local fullattrs
   if not perms then
      fullattrs = lfs.attributes(src, "permissions")
   end
   if fullattrs and posix_ok then
      return posix.chmod(dest, fullattrs)
   else
      if cfg.is_platform("unix") then
         if not perms then
            perms = fullattrs:match("x") and "exec" or "read"
         end
         return fs.set_permissions(dest, perms, "all")
      else
         return true
      end
   end
end

--- Implementation function for recursive copy of directory contents.
-- Assumes paths are normalized.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @param perms string ("read" or "exec") or nil: Optional permissions.
-- If not given, permissions of the source are copied over to the destination.
-- @return boolean or (boolean, string): true on success, false on failure
local function recursive_copy(src, dest, perms)
   local srcmode = lfs.attributes(src, "mode")

   if srcmode == "file" then
      local ok = fs.copy(src, dest, perms)
      if not ok then return false end
   elseif srcmode == "directory" then
      local subdir = dir.path(dest, dir.base_name(src))
      local ok, err = fs.make_dir(subdir)
      if not ok then return nil, err end
      if pcall(lfs.dir, src) == false then
         return false
      end
      for file in lfs.dir(src) do
         if file ~= "." and file ~= ".." then
            local ok = recursive_copy(dir.path(src, file), subdir, perms)
            if not ok then return false end
         end
      end
   end
   return true
end

--- Recursively copy the contents of a directory.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @param perms string ("read" or "exec") or nil: Optional permissions.
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function fs_lua.copy_contents(src, dest, perms)
   assert(src)
   assert(dest)
   src = dir.normalize(src)
   dest = dir.normalize(dest)
   if not fs.is_dir(src) then
      return false, src .. " is not a directory"
   end
   if pcall(lfs.dir, src) == false then
      return false, "Permission denied"
   end
   for file in lfs.dir(src) do
      if file ~= "." and file ~= ".." then
         local ok = recursive_copy(dir.path(src, file), dest, perms)
         if not ok then
            return false, "Failed copying "..src.." to "..dest
         end
      end
   end
   return true
end

--- Implementation function for recursive removal of directories.
-- Assumes paths are normalized.
-- @param name string: Pathname of file
-- @return boolean or (boolean, string): true on success,
-- or nil and an error message on failure.
local function recursive_delete(name)
   local ok = os.remove(name)
   if ok then return true end
   local pok, ok, err = pcall(function()
      for file in lfs.dir(name) do
         if file ~= "." and file ~= ".." then
            local ok, err = recursive_delete(dir.path(name, file))
            if not ok then return nil, err end
         end
      end
      local ok, err = lfs.rmdir(name)
      return ok, (not ok) and err
   end)
   if pok then
      return ok, err
   else
      return pok, ok
   end
end

--- Delete a file or a directory and all its contents.
-- @param name string: Pathname of source
-- @return nil
function fs_lua.delete(name)
   name = dir.normalize(name)
   recursive_delete(name)
end

--- Internal implementation function for fs.dir.
-- Yields a filename on each iteration.
-- @param at string: directory to list
-- @return nil or (nil and string): an error message on failure
function fs_lua.dir_iterator(at)
   local pok, iter, arg = pcall(lfs.dir, at)
   if not pok then
      return nil, iter
   end
   for file in iter, arg do
      if file ~= "." and file ~= ".." then
         coroutine.yield(file)
      end
   end
end

--- Implementation function for recursive find.
-- Assumes paths are normalized.
-- @param cwd string: Current working directory in recursion.
-- @param prefix string: Auxiliary prefix string to form pathname.
-- @param result table: Array of strings where results are collected.
local function recursive_find(cwd, prefix, result)
   local pok, iter, arg = pcall(lfs.dir, cwd)
   if not pok then
      return nil
   end
   for file in iter, arg do
      if file ~= "." and file ~= ".." then
         local item = prefix .. file
         table.insert(result, item)
         local pathname = dir.path(cwd, file)
         if lfs.attributes(pathname, "mode") == "directory" then
            recursive_find(pathname, item .. dir_sep, result)
         end
      end
   end
end

--- Recursively scan the contents of a directory.
-- @param at string or nil: directory to scan (will be the current
-- directory if none is given).
-- @return table: an array of strings with the filenames representing
-- the contents of a directory.
function fs_lua.find(at)
   assert(type(at) == "string" or not at)
   if not at then
      at = fs.current_dir()
   end
   at = dir.normalize(at)
   local result = {}
   recursive_find(at, "", result)
   return result
end

--- Test for existence of a file.
-- @param file string: filename to test
-- @return boolean: true if file exists, false otherwise.
function fs_lua.exists(file)
   assert(file)
   file = dir.normalize(file)
   return type(lfs.attributes(file)) == "table"
end

--- Test is pathname is a directory.
-- @param file string: pathname to test
-- @return boolean: true if it is a directory, false otherwise.
function fs_lua.is_dir(file)
   assert(file)
   file = dir.normalize(file)
   return lfs.attributes(file, "mode") == "directory"
end

--- Test is pathname is a regular file.
-- @param file string: pathname to test
-- @return boolean: true if it is a file, false otherwise.
function fs_lua.is_file(file)
   assert(file)
   file = dir.normalize(file)
   return lfs.attributes(file, "mode") == "file"
end

-- Set access and modification times for a file.
-- @param filename File to set access and modification times for.
-- @param time may be a number containing the format returned
-- by os.time, or a table ready to be processed via os.time; if
-- nil, current time is assumed.
function fs_lua.set_time(file, time)
   assert(time == nil or type(time) == "table" or type(time) == "number")
   file = dir.normalize(file)
   if type(time) == "table" then
      time = os.time(time)
   end
   return lfs.touch(file, time)
end

else -- if not lfs_ok

function fs_lua.exists(file)
   assert(file)
   -- check if file exists by attempting to open it
   return util.exists(fs.absolute_name(file))
end

function fs_lua.file_age(_)
   return math.huge
end

end

---------------------------------------------------------------------
-- lua-bz2 functions
---------------------------------------------------------------------

if bz2_ok then

local function bunzip2_string(data)
   local decompressor = bz2.initDecompress()
   local output, err = decompressor:update(data)
   if not output then
      return nil, err
   end
   decompressor:close()
   return output
end

--- Uncompresses a .bz2 file.
-- @param infile string: pathname of .bz2 file to be extracted.
-- @param outfile string or nil: pathname of output file to be produced.
-- If not given, name is derived from input file.
-- @return boolean: true on success; nil and error message on failure.
function fs_lua.bunzip2(infile, outfile)
   assert(type(infile) == "string")
   assert(outfile == nil or type(outfile) == "string")
   if not outfile then
      outfile = infile:gsub("%.bz2$", "")
   end

   return fs.filter_file(bunzip2_string, infile, outfile)
end

end

---------------------------------------------------------------------
-- luarocks.tools.zip functions
---------------------------------------------------------------------

if zip_ok then

function fs_lua.zip(zipfile, ...)
   return zip.zip(zipfile, ...)
end

function fs_lua.unzip(zipfile)
   return zip.unzip(zipfile)
end

function fs_lua.gunzip(infile, outfile)
   return zip.gunzip(infile, outfile)
end

end

---------------------------------------------------------------------
-- LuaSocket functions
---------------------------------------------------------------------

if socket_ok then

local ltn12 = require("ltn12")
local luasec_ok, https = pcall(require, "ssl.https")

if luasec_ok and not vers.compare_versions(https._VERSION, "1.0.3") then
   luasec_ok = false
   https = nil
end

local redirect_protocols = {
   http = http,
   https = luasec_ok and https,
}

local function request(url, method, http, loop_control)  -- luacheck: ignore 431
   local result = {}

   if cfg.verbose then
      print(method, url)
   end

   local proxy = os.getenv("http_proxy")
   if type(proxy) ~= "string" then proxy = nil end
   -- LuaSocket's http.request crashes when given URLs missing the scheme part.
   if proxy and not proxy:find("://") then
      proxy = "http://" .. proxy
   end

   if cfg.show_downloads then
      io.write(method.." "..url.." ...\n")
   end
   local dots = 0
   if cfg.connection_timeout and cfg.connection_timeout > 0 then
      http.TIMEOUT = cfg.connection_timeout
   end
   local res, status, headers, err = http.request {
      url = url,
      proxy = proxy,
      method = method,
      redirect = false,
      sink = ltn12.sink.table(result),
      step = cfg.show_downloads and function(...)
         io.write(".")
         io.flush()
         dots = dots + 1
         if dots == 70 then
            io.write("\n")
            dots = 0
         end
         return ltn12.pump.step(...)
      end,
      headers = {
         ["user-agent"] = cfg.user_agent.." via LuaSocket"
      },
   }
   if cfg.show_downloads then
      io.write("\n")
   end
   if not res then
      return nil, status
   elseif status == 301 or status == 302 then
      local location = headers.location
      if location then
         local protocol, rest = dir.split_url(location)
         if redirect_protocols[protocol] then
            if not loop_control then
               loop_control = {}
            elseif loop_control[location] then
               return nil, "Redirection loop -- broken URL?"
            end
            loop_control[url] = true
            return request(location, method, redirect_protocols[protocol], loop_control)
         else
            return nil, "URL redirected to unsupported protocol - install luasec >= 1.1 to get HTTPS support.", "https"
         end
      end
      return nil, err
   elseif status ~= 200 then
      return nil, err
   else
      return result, status, headers, err
   end
end

local function write_timestamp(filename, data)
   local fd = io.open(filename, "w")
   if fd then
      fd:write(data)
      fd:close()
   end
end

local function read_timestamp(filename)
   local fd = io.open(filename, "r")
   if fd then
      local data = fd:read("*a")
      fd:close()
      return data
   end
end

local function fail_with_status(filename, status, headers)
   write_timestamp(filename .. ".unixtime", os.time())
   write_timestamp(filename .. ".status", status)
   return nil, status, headers
end

-- @param url string: URL to fetch.
-- @param filename string: local filename of the file to fetch.
-- @param http table: The library to use (http from LuaSocket or LuaSec)
-- @param cache boolean: Whether to use a `.timestamp` file to check
-- via the HTTP Last-Modified header if the full download is needed.
-- @return (boolean | (nil, string, string?)): True if successful, or
-- nil, error message and optionally HTTPS error in case of errors.
local function http_request(url, filename, http, cache)  -- luacheck: ignore 431
   if cache then
      local status = read_timestamp(filename..".status")
      local timestamp = read_timestamp(filename..".timestamp")
      if status or timestamp then
         local unixtime = read_timestamp(filename..".unixtime")
         if tonumber(unixtime) then
            local diff = os.time() - tonumber(unixtime)
            if status then
               if diff < cfg.cache_fail_timeout then
                  return nil, status, {}
               end
            else
               if diff < cfg.cache_timeout then
                  return true, nil, nil, true
               end
            end
         end

         local result, status, headers, err = request(url, "HEAD", http)  -- luacheck: ignore 421
         if not result then
            return fail_with_status(filename, status, headers)
         end
         if status == 200 and headers["last-modified"] == timestamp then
            write_timestamp(filename .. ".unixtime", os.time())
            return true, nil, nil, true
         end
      end
   end
   local result, status, headers, err = request(url, "GET", http)
   if not result then
      if status then
         return fail_with_status(filename, status, headers)
      end
   end
   if cache and headers["last-modified"] then
      write_timestamp(filename .. ".timestamp", headers["last-modified"])
      write_timestamp(filename .. ".unixtime", os.time())
   end
   local file = io.open(filename, "wb")
   if not file then return nil, 0, {} end
   for _, data in ipairs(result) do
      file:write(data)
   end
   file:close()
   return true
end

local function ftp_request(url, filename)
   local content, err = ftp.get(url)
   if not content then
      return false, err
   end
   local file = io.open(filename, "wb")
   if not file then return false, err end
   file:write(content)
   file:close()
   return true
end

local downloader_warning = false

--- Download a remote file.
-- @param url string: URL to be fetched.
-- @param filename string or nil: this function attempts to detect the
-- resulting local filename of the remote file as the basename of the URL;
-- if that is not correct (due to a redirection, for example), the local
-- filename can be given explicitly as this second argument.
-- @return (boolean, string, boolean):
-- In case of success:
-- * true
-- * a string with the filename
-- * true if the file was retrieved from local cache
-- In case of failure:
-- * false
-- * error message
function fs_lua.download(url, filename, cache)
   assert(type(url) == "string")
   assert(type(filename) == "string" or not filename)

   filename = fs.absolute_name(filename or dir.base_name(url))

   -- delegate to the configured downloader so we don't have to deal with whitelists
   if os.getenv("no_proxy") then
      return fs.use_downloader(url, filename, cache)
   end

   local ok, err, https_err, from_cache
   if util.starts_with(url, "http:") then
      ok, err, https_err, from_cache = http_request(url, filename, http, cache)
   elseif util.starts_with(url, "ftp:") then
      ok, err = ftp_request(url, filename)
   elseif util.starts_with(url, "https:") then
      -- skip LuaSec when proxy is enabled since it is not supported
      if luasec_ok and not os.getenv("https_proxy") then
         local _
         ok, err, _, from_cache = http_request(url, filename, https, cache)
      else
         https_err = true
      end
   else
      err = "Unsupported protocol"
   end
   if https_err then
      local downloader, err = fs.which_tool("downloader")
      if not downloader then
         return nil, err
      end
      if not downloader_warning then
         util.warning("falling back to "..downloader.." - install luasec >= 1.1 to get native HTTPS support")
         downloader_warning = true
      end
      return fs.use_downloader(url, filename, cache)
   elseif not ok then
      return nil, err, "network"
   end
   return true, filename, from_cache
end

else --...if socket_ok == false then

function fs_lua.download(url, filename, cache)
   return fs.use_downloader(url, filename, cache)
end

end
---------------------------------------------------------------------
-- MD5 functions
---------------------------------------------------------------------

if md5_ok then

-- Support the interface of lmd5 by lhf in addition to md5 by Roberto
-- and the keplerproject.
if not md5.sumhexa and md5.digest then
   md5.sumhexa = function(msg)
      return md5.digest(msg)
   end
end

if md5.sumhexa then

--- Get the MD5 checksum for a file.
-- @param file string: The file to be computed.
-- @return string: The MD5 checksum or nil + error
function fs_lua.get_md5(file)
   file = fs.absolute_name(file)
   local file_handler = io.open(file, "rb")
   if not file_handler then return nil, "Failed to open file for reading: "..file end
   local computed = md5.sumhexa(file_handler:read("*a"))
   file_handler:close()
   if computed then return computed end
   return nil, "Failed to compute MD5 hash for file "..file
end

end
end

---------------------------------------------------------------------
-- POSIX functions
---------------------------------------------------------------------

function fs_lua._unix_rwx_to_number(rwx, neg)
   local num = 0
   neg = neg or false
   for i = 1, 9 do
      local c = rwx:sub(10 - i, 10 - i) == "-"
      if neg == c then
         num = num + 2^(i-1)
      end
   end
   return math.floor(num)
end

if posix_ok then

local octal_to_rwx = {
   ["0"] = "---",
   ["1"] = "--x",
   ["2"] = "-w-",
   ["3"] = "-wx",
   ["4"] = "r--",
   ["5"] = "r-x",
   ["6"] = "rw-",
   ["7"] = "rwx",
}

do
   local umask_cache
   function fs_lua._unix_umask()
      if umask_cache then
         return umask_cache
      end
      -- LuaPosix (as of 34.0.4) only returns the umask as rwx
      local rwx = posix.umask()
      local num = fs_lua._unix_rwx_to_number(rwx, true)
      umask_cache = ("%03o"):format(num)
      return umask_cache
   end
end

function fs_lua.set_permissions(filename, mode, scope)
   local perms
   if mode == "read" and scope == "user" then
      perms = fs._unix_moderate_permissions("600")
   elseif mode == "exec" and scope == "user" then
      perms = fs._unix_moderate_permissions("700")
   elseif mode == "read" and scope == "all" then
      perms = fs._unix_moderate_permissions("644")
   elseif mode == "exec" and scope == "all" then
      perms = fs._unix_moderate_permissions("755")
   else
      return false, "Invalid permission " .. mode .. " for " .. scope
   end

   -- LuaPosix (as of 5.1.15) does not support octal notation...
   local new_perms = {}
   for c in perms:sub(-3):gmatch(".") do
      table.insert(new_perms, octal_to_rwx[c])
   end
   perms = table.concat(new_perms)
   local err = posix.chmod(filename, perms)
   return err == 0
end

function fs_lua.current_user()
   return posix.getpwuid(posix.geteuid()).pw_name
end

function fs_lua.is_superuser()
   return posix.geteuid() == 0
end

-- This call is not available on all systems, see #677
if posix.mkdtemp then

--- Create a temporary directory.
-- @param name_pattern string: name pattern to use for avoiding conflicts
-- when creating temporary directory.
-- @return string or (nil, string): name of temporary directory or (nil, error message) on failure.
function fs_lua.make_temp_dir(name_pattern)
   assert(type(name_pattern) == "string")

   return posix.mkdtemp(temp_dir_pattern(name_pattern) .. "-XXXXXX")
end

end -- if posix.mkdtemp

end

---------------------------------------------------------------------
-- Other functions
---------------------------------------------------------------------

if not fs_lua.make_temp_dir then

function fs_lua.make_temp_dir(name_pattern)
   assert(type(name_pattern) == "string")

   local ok, err
   for _ = 1, 3 do
      local name = temp_dir_pattern(name_pattern) .. tostring(math.random(10000000))
      ok, err = fs.make_dir(name)
      if ok then
         return name
      end
   end

   return nil, err
end

end

--- Apply a patch.
-- @param patchname string: The filename of the patch.
-- @param patchdata string or nil: The actual patch as a string.
-- @param create_delete boolean: Support creating and deleting files in a patch.
function fs_lua.apply_patch(patchname, patchdata, create_delete)
   local p, all_ok = patch.read_patch(patchname, patchdata)
   if not all_ok then
      return nil, "Failed reading patch "..patchname
   end
   if p then
      return patch.apply_patch(p, 1, create_delete)
   end
end

--- Move a file.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @param perms string ("read" or "exec") or nil: Permissions for destination
-- file or nil to use the source file permissions.
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function fs_lua.move(src, dest, perms)
   assert(src and dest)
   if fs.exists(dest) and not fs.is_dir(dest) then
      return false, "File already exists: "..dest
   end
   local ok, err = fs.copy(src, dest, perms)
   if not ok then
      return false, err
   end
   fs.delete(src)
   if fs.exists(src) then
      return false, "Failed move: could not delete "..src.." after copy."
   end
   return true
end

local function get_local_tree()
   for _, tree in ipairs(cfg.rocks_trees) do
      if type(tree) == "table" and tree.name == "user" then
         return fs.absolute_name(tree.root)
      end
   end
end

local function is_local_tree_in_env(local_tree)
   local lua_path
   if _VERSION == "Lua 5.1" then
      lua_path = os.getenv("LUA_PATH")
   else
      lua_path = os.getenv("LUA_PATH_" .. _VERSION:sub(5):gsub("%.", "_"))
                 or os.getenv("LUA_PATH")
   end
   if lua_path and lua_path:match(local_tree, 1, true) then
      return true
   end
end

--- Check if user has write permissions for the command.
-- Assumes the configuration variables under cfg have been previously set up.
-- @param args table: the args table passed to run() drivers.
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function fs_lua.check_command_permissions(args)
   local ok = true
   local err = ""
   if args._command_permissions_checked then
      return true
   end
   for _, directory in ipairs { cfg.rocks_dir, cfg.deploy_lua_dir, cfg.deploy_bin_dir, cfg.deploy_lua_dir } do
      if fs.exists(directory) then
         if not fs.is_writable(directory) then
            ok = false
            err = "Your user does not have write permissions in " .. directory
            break
         end
      else
         local root = fs.root_of(directory)
         local parent = directory
         repeat
            parent = dir.dir_name(parent)
            if parent == "" then
               parent = root
            end
         until parent == root or fs.exists(parent)
         if not fs.is_writable(parent) then
            ok = false
            err = directory.." does not exist\nand your user does not have write permissions in " .. parent
            break
         end
      end
   end
   if ok then
      args._command_permissions_checked = true
      return true
   else
      if args["local"] or cfg.local_by_default then
         err = err .. "\n\nPlease check your permissions.\n"
      else
         local local_tree = get_local_tree()
         if local_tree then
            err = err .. "\n\nYou may want to run as a privileged user,"
                      .. "\nor use --local to install into your local tree at " .. local_tree
                      .. "\nor run 'luarocks config local_by_default true' to make --local the default.\n"

            if not is_local_tree_in_env(local_tree) then
               err = err .. "\n(You may need to configure your Lua package paths\nto use the local tree, see 'luarocks path --help')\n"
            end
         else
            err = err .. "\n\nYou may want to run as a privileged user.\n"
         end
      end
      return nil, err
   end
end

--- Check whether a file is a Lua script
-- When the file can be successfully compiled by the configured
-- Lua interpreter, it's considered to be a valid Lua file.
-- @param filename filename of file to check
-- @return boolean true, if it is a Lua script, false otherwise
function fs_lua.is_lua(filename)
  filename = filename:gsub([[%\]],"/")   -- normalize on fw slash to prevent escaping issues
  local lua = fs.Q(cfg.variables.LUA)  -- get lua interpreter configured
  -- execute on configured interpreter, might not be the same as the interpreter LR is run on
  local result = fs.execute_string(lua..[[ -e "if loadfile(']]..filename..[[') then os.exit(0) else os.exit(1) end"]])
  return (result == true)
end

--- Unpack an archive.
-- Extract the contents of an archive, detecting its format by
-- filename extension.
-- @param archive string: Filename of archive.
-- @return boolean or (boolean, string): true on success, false and an error message on failure.
function fs_lua.unpack_archive(archive)
   assert(type(archive) == "string")

   local ok, err
   archive = fs.absolute_name(archive)
   if archive:match("%.tar%.gz$") then
      local tar_filename = archive:gsub("%.gz$", "")
      ok, err = fs.gunzip(archive, tar_filename)
      if ok then
         ok, err = tar.untar(tar_filename, ".")
      end
   elseif archive:match("%.tgz$") then
      local tar_filename = archive:gsub("%.tgz$", ".tar")
      ok, err = fs.gunzip(archive, tar_filename)
      if ok then
         ok, err = tar.untar(tar_filename, ".")
      end
   elseif archive:match("%.tar%.bz2$") then
      local tar_filename = archive:gsub("%.bz2$", "")
      ok, err = fs.bunzip2(archive, tar_filename)
      if ok then
         ok, err = tar.untar(tar_filename, ".")
      end
   elseif archive:match("%.zip$") then
      ok, err = fs.unzip(archive)
   elseif archive:match("%.lua$") or archive:match("%.c$") then
      -- Ignore .lua and .c files; they don't need to be extracted.
      return true
   else
      return false, "Couldn't extract archive "..archive..": unrecognized filename extension"
   end
   if not ok then
      return false, "Failed extracting "..archive..": "..err
   end
   return true
end

return fs_lua
