--- Windows implementation of filesystem and platform abstractions.
-- Download http://unxutils.sourceforge.net/ for Windows GNU utilities
-- used by this module.
local win32 = {}

local fs = require("luarocks.fs")

local cfg = require("luarocks.core.cfg")
local dir = require("luarocks.dir")
local path = require("luarocks.path")
local util = require("luarocks.util")

-- Monkey patch io.popen and os.execute to make sure quoting
-- works as expected.
-- See http://lua-users.org/lists/lua-l/2013-11/msg00367.html
local _prefix = "type NUL && "
local _popen, _execute = io.popen, os.execute

-- luacheck: push globals io os
io.popen = function(cmd, ...) return _popen(_prefix..cmd, ...) end
os.execute = function(cmd, ...) return _execute(_prefix..cmd, ...) end
-- luacheck: pop

--- Annotate command string for quiet execution.
-- @param cmd string: A command-line string.
-- @return string: The command-line, with silencing annotation.
function win32.quiet(cmd)
   return cmd.." 2> NUL 1> NUL"
end

--- Annotate command string for execution with quiet stderr.
-- @param cmd string: A command-line string.
-- @return string: The command-line, with stderr silencing annotation.
function win32.quiet_stderr(cmd)
   return cmd.." 2> NUL"
end

function win32.execute_env(env, command, ...)
   assert(type(command) == "string")
   local cmdstr = {}
   for var, val in pairs(env) do
      table.insert(cmdstr, fs.export_cmd(var, val))
   end
   table.insert(cmdstr, fs.quote_args(command, ...))
   return fs.execute_string(table.concat(cmdstr, " & "))
end

-- Split path into drive, root and the rest.
-- Example: "c:\\hello\\world" becomes "c:" "\\" "hello\\world"
-- if any part is missing from input, it becomes an empty string.
local function split_root(pathname)
   local drive = ""
   local root = ""
   local rest

   local unquoted = pathname:match("^['\"](.*)['\"]$")
   if unquoted then
      pathname = unquoted
   end

   if pathname:match("^.:") then
      drive = pathname:sub(1, 2)
      pathname = pathname:sub(3)
   end

   if pathname:match("^[\\/]") then
      root = pathname:sub(1, 1)
      rest = pathname:sub(2)
   else
      rest = pathname
   end

   return drive, root, rest
end

--- Quote argument for shell processing. Fixes paths on Windows.
-- Adds double quotes and escapes.
-- @param arg string: Unquoted argument.
-- @return string: Quoted argument.
function win32.Q(arg)
   assert(type(arg) == "string")
   -- Use Windows-specific directory separator for paths.
   -- Paths should be converted to absolute by now.
   local drive, root, rest = split_root(arg)
   if root ~= "" then
      arg = arg:gsub("/", "\\")
   end
   if arg == "\\" then
      return '\\' -- CHDIR needs special handling for root dir
   end
   -- URLs and anything else
   arg = arg:gsub('\\(\\*)"', '\\%1%1"')
   arg = arg:gsub('\\+$', '%0%0')
   arg = arg:gsub('"', '\\"')
   arg = arg:gsub('(\\*)%%', '%1%1"%%"')
   return '"' .. arg .. '"'
end

--- Quote argument for shell processing in batch files.
-- Adds double quotes and escapes.
-- @param arg string: Unquoted argument.
-- @return string: Quoted argument.
function win32.Qb(arg)
   assert(type(arg) == "string")
   -- Use Windows-specific directory separator for paths.
   -- Paths should be converted to absolute by now.
   local drive, root, rest = split_root(arg)
   if root ~= "" then
      arg = arg:gsub("/", "\\")
   end
   if arg == "\\" then
      return '\\' -- CHDIR needs special handling for root dir
   end
   -- URLs and anything else
   arg = arg:gsub('\\(\\*)"', '\\%1%1"')
   arg = arg:gsub('\\+$', '%0%0')
   arg = arg:gsub('"', '\\"')
   arg = arg:gsub('%%', '%%%%')
   return '"' .. arg .. '"'
end

--- Return an absolute pathname from a potentially relative one.
-- @param pathname string: pathname to convert.
-- @param relative_to string or nil: path to prepend when making
-- pathname absolute, or the current dir in the dir stack if
-- not given.
-- @return string: The pathname converted to absolute.
function win32.absolute_name(pathname, relative_to)
   assert(type(pathname) == "string")
   assert(type(relative_to) == "string" or not relative_to)

   relative_to = (relative_to or fs.current_dir()):gsub("[\\/]*$", "")
   local drive, root, rest = split_root(pathname)
   if root:match("[\\/]$") then
      -- It's an absolute path already. Ensure is not quoted.
      return dir.normalize(drive .. root .. rest)
   else
      -- It's a relative path, join it with base path.
      -- This drops drive letter from paths like "C:foo".
      return dir.path(relative_to, rest)
   end
end

--- Return the root directory for the given path.
-- For example, for "c:\hello", returns "c:\"
-- @param pathname string: pathname to use.
-- @return string: The root of the given pathname.
function win32.root_of(pathname)
   local drive, root, rest = split_root(fs.absolute_name(pathname))
   return drive .. root
end

--- Create a wrapper to make a script executable from the command-line.
-- @param script string: Pathname of script to be made executable.
-- @param target string: wrapper target pathname (without wrapper suffix).
-- @param name string: rock name to be used in loader context.
-- @param version string: rock version to be used in loader context.
-- @return boolean or (nil, string): True if succeeded, or nil and
-- an error message.
function win32.wrap_script(script, target, deps_mode, name, version, ...)
   assert(type(script) == "string" or not script)
   assert(type(target) == "string")
   assert(type(deps_mode) == "string")
   assert(type(name) == "string" or not name)
   assert(type(version) == "string" or not version)

   local wrapper = io.open(target, "wb")
   if not wrapper then
      return nil, "Could not open "..target.." for writing."
   end

   local lpath, lcpath = path.package_paths(deps_mode)

   local luainit = {
      "package.path="..util.LQ(lpath..";").."..package.path",
      "package.cpath="..util.LQ(lcpath..";").."..package.cpath",
   }

   local remove_interpreter = false
   local base = dir.base_name(target):gsub("%..*$", "")
   if base == "luarocks" or base == "luarocks-admin" then
      if cfg.is_binary then
         remove_interpreter = true
      end
      luainit = {
         "package.path="..util.LQ(package.path),
         "package.cpath="..util.LQ(package.cpath),
      }
   end

   if name and version then
      local addctx = "local k,l,_=pcall(require,'luarocks.loader') _=k " ..
                     "and l.add_context('"..name.."','"..version.."')"
      table.insert(luainit, addctx)
   end

   local argv = {
      fs.Qb(cfg.variables["LUA"]),
      "-e",
      fs.Qb(table.concat(luainit, ";")),
      script and fs.Qb(script) or "%I%",
      ...
   }
   if remove_interpreter then
      table.remove(argv, 1)
      table.remove(argv, 1)
      table.remove(argv, 1)
   end

   wrapper:write("@echo off\r\n")
   wrapper:write("setlocal\r\n")
   if not script then
      wrapper:write([[IF "%*"=="" (set I=-i) ELSE (set I=)]] .. "\r\n")
   end
   wrapper:write("set "..fs.Qb("LUAROCKS_SYSCONFDIR="..cfg.sysconfdir) .. "\r\n")
   wrapper:write(table.concat(argv, " ") .. " %*\r\n")
   wrapper:write("exit /b %ERRORLEVEL%\r\n")
   wrapper:close()
   return true
end

function win32.is_actual_binary(name)
   name = name:lower()
   if name:match("%.bat$") or name:match("%.exe$") then
      return true
   end
   return false
end

function win32.copy_binary(filename, dest)
   local ok, err = fs.copy(filename, dest)
   if not ok then
      return nil, err
   end
   local exe_pattern = "%.[Ee][Xx][Ee]$"
   local base = dir.base_name(filename)
   dest = dir.dir_name(dest)
   if base:match(exe_pattern) then
      base = base:gsub(exe_pattern, ".lua")
      local helpname = dest.."\\"..base
      local helper = io.open(helpname, "w")
      if not helper then
         return nil, "Could not open "..helpname.." for writing."
      end
      helper:write('package.path=\"'..package.path:gsub("\\","\\\\")..';\"..package.path\n')
      helper:write('package.cpath=\"'..package.path:gsub("\\","\\\\")..';\"..package.cpath\n')
      helper:close()
   end
   return true
end

--- Move a file on top of the other.
-- The new file ceases to exist under its original name,
-- and takes over the name of the old file.
-- On Windows this is done by removing the original file and
-- renaming the new file to its original name.
-- @param old_file The name of the original file,
-- which will be the new name of new_file.
-- @param new_file The name of the new file,
-- which will replace old_file.
-- @return boolean or (nil, string): True if succeeded, or nil and
-- an error message.
function win32.replace_file(old_file, new_file)
   os.remove(old_file)
   return os.rename(new_file, old_file)
end

function win32.is_dir(file)
   file = fs.absolute_name(file)
   file = dir.normalize(file)
   local fd, _, code = io.open(file, "r")
   if code == 13 then -- directories return "Permission denied"
      fd, _, code = io.open(file .. "\\", "r")
      if code == 2 then -- directories return 2, files return 22
         return true
      end
   end
   if fd then
      fd:close()
   end
   return false
end

function win32.is_file(file)
   file = fs.absolute_name(file)
   file = dir.normalize(file)
   local fd, _, code = io.open(file, "r")
   if code == 13 then -- if "Permission denied"
      fd, _, code = io.open(file .. "\\", "r")
      if code == 2 then -- directories return 2, files return 22
         return false
      elseif code == 22 then
         return true
      end
   end
   if fd then
      fd:close()
      return true
   end
   return false
end

--- Test is file/dir is writable.
-- Warning: testing if a file/dir is writable does not guarantee
-- that it will remain writable and therefore it is no replacement
-- for checking the result of subsequent operations.
-- @param file string: filename to test
-- @return boolean: true if file exists, false otherwise.
function win32.is_writable(file)
   assert(file)
   file = dir.normalize(file)
   local result
   local tmpname = 'tmpluarockstestwritable.deleteme'
   if fs.is_dir(file) then
      local file2 = dir.path(file, tmpname)
      local fh = io.open(file2, 'wb')
      result = fh ~= nil
      if fh then fh:close() end
      if result then
         -- the above test might give a false positive when writing to
         -- c:\program files\ because of VirtualStore redirection on Vista and up
         -- So check whether it's really there
         result = fs.exists(file2)
      end
      os.remove(file2)
   else
      local fh = io.open(file, 'r+b')
      result = fh ~= nil
      if fh then fh:close() end
   end
   return result
end

function win32.tmpname()
   local name = os.tmpname()
   local tmp = os.getenv("TMP")
   if tmp and name:sub(1, #tmp) ~= tmp then
      name = (tmp .. "\\" .. name):gsub("\\+", "\\")
   end
   return name
end

function win32.current_user()
   return os.getenv("USERNAME")
end

function win32.is_superuser()
   return false
end

function win32.export_cmd(var, val)
   return ("SET %s"):format(fs.Q(var.."="..val))
end

function win32.system_cache_dir()
   return dir.path(fs.system_temp_dir(), "cache")
end

function win32.search_in_path(program)
   if program:match("\\") then
      local fd = io.open(dir.path(program), "r")
      if fd then
         fd:close()
         return true, program
      end

      return false
   end

   if not program:lower():match("exe$") then
      program = program .. ".exe"
   end

   for d in (os.getenv("PATH") or ""):gmatch("([^;]+)") do
      local fd = io.open(dir.path(d, program), "r")
      if fd then
         fd:close()
         return true, d
      end
   end
   return false
end

return win32
