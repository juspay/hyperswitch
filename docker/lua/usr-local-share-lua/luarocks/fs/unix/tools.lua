
--- fs operations implemented with third-party tools for Unix platform abstractions.
local tools = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local cfg = require("luarocks.core.cfg")

local vars = setmetatable({}, { __index = function(_,k) return cfg.variables[k] end })

--- Adds prefix to command to make it run from a directory.
-- @param directory string: Path to a directory.
-- @param cmd string: A command-line string.
-- @return string: The command-line with prefix.
function tools.command_at(directory, cmd)
   return "cd " .. fs.Q(fs.absolute_name(directory)) .. " && " .. cmd
end

--- Create a directory if it does not already exist.
-- If any of the higher levels in the path name does not exist
-- too, they are created as well.
-- @param directory string: pathname of directory to create.
-- @return boolean: true on success, false on failure.
function tools.make_dir(directory)
   assert(directory)
   local ok, err = fs.execute(vars.MKDIR.." -p", directory)
   if not ok then
      err = "failed making directory "..directory
   end
   return ok, err
end

--- Remove a directory if it is empty.
-- Does not return errors (for example, if directory is not empty or
-- if already does not exist)
-- @param directory string: pathname of directory to remove.
function tools.remove_dir_if_empty(directory)
   assert(directory)
   fs.execute_quiet(vars.RMDIR, directory)
end

--- Remove a directory if it is empty.
-- Does not return errors (for example, if directory is not empty or
-- if already does not exist)
-- @param directory string: pathname of directory to remove.
function tools.remove_dir_tree_if_empty(directory)
   assert(directory)
   fs.execute_quiet(vars.RMDIR, "-p", directory)
end

--- Copy a file.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @param perm string ("read" or "exec") or nil: Permissions for destination
-- file or nil to use the source permissions
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function tools.copy(src, dest, perm)
   assert(src and dest)
   if fs.execute(vars.CP, src, dest) then
      if perm then
         if fs.is_dir(dest) then
            dest = dir.path(dest, dir.base_name(src))
         end
         if fs.set_permissions(dest, perm, "all") then
            return true
         else
            return false, "Failed setting permissions of "..dest
         end
      end
      return true
   else
      return false, "Failed copying "..src.." to "..dest
   end
end

--- Recursively copy the contents of a directory.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function tools.copy_contents(src, dest)
   assert(src and dest)
   if not fs.is_dir(src) then
      return false, src .. " is not a directory"
   end
   if fs.make_dir(dest) and fs.execute_quiet(vars.CP.." -pPR "..fs.Q(src).."/* "..fs.Q(dest)) then
      return true
   else
      return false, "Failed copying "..src.." to "..dest
   end
end
--- Delete a file or a directory and all its contents.
-- For safety, this only accepts absolute paths.
-- @param arg string: Pathname of source
-- @return nil
function tools.delete(arg)
   assert(arg)
   assert(arg:sub(1,1) == "/")
   fs.execute_quiet(vars.RM, "-rf", arg)
end

--- Recursively scan the contents of a directory.
-- @param at string or nil: directory to scan (will be the current
-- directory if none is given).
-- @return table: an array of strings with the filenames representing
-- the contents of a directory.
function tools.find(at)
   assert(type(at) == "string" or not at)
   if not at then
      at = fs.current_dir()
   end
   if not fs.is_dir(at) then
      return {}
   end
   local result = {}
   local pipe = io.popen(fs.command_at(at, fs.quiet_stderr(vars.FIND.." *")))
   for file in pipe:lines() do
      table.insert(result, file)
   end
   pipe:close()
   return result
end

--- Compress files in a .zip archive.
-- @param zipfile string: pathname of .zip archive to be created.
-- @param ... Filenames to be stored in the archive are given as
-- additional arguments.
-- @return boolean: true on success, nil and error message on failure.
function tools.zip(zipfile, ...)
   local ok, err = fs.is_tool_available(vars.ZIP, "zip")
   if not ok then
      return nil, err
   end
   if fs.execute_quiet(vars.ZIP.." -r", zipfile, ...) then
      return true
   else
      return nil, "failed compressing " .. zipfile
   end
end

--- Uncompress files from a .zip archive.
-- @param zipfile string: pathname of .zip archive to be extracted.
-- @return boolean: true on success, nil and error message on failure.
function tools.unzip(zipfile)
   assert(zipfile)
   local ok, err = fs.is_tool_available(vars.UNZIP, "unzip")
   if not ok then
      return nil, err
   end
   if fs.execute_quiet(vars.UNZIP, zipfile) then
      return true
   else
      return nil, "failed extracting " .. zipfile
   end
end

local function uncompress(default_ext, program, infile, outfile)
   assert(type(infile) == "string")
   assert(outfile == nil or type(outfile) == "string")
   if not outfile then
      outfile = infile:gsub("%."..default_ext.."$", "")
   end
   if fs.execute(fs.Q(program).." -c "..fs.Q(infile).." > "..fs.Q(outfile)) then
      return true
   else
      return nil, "failed extracting " .. infile
   end
end

--- Uncompresses a .gz file.
-- @param infile string: pathname of .gz file to be extracted.
-- @param outfile string or nil: pathname of output file to be produced.
-- If not given, name is derived from input file.
-- @return boolean: true on success; nil and error message on failure.
function tools.gunzip(infile, outfile)
   return uncompress("gz", "gunzip", infile, outfile)
end

--- Uncompresses a .bz2 file.
-- @param infile string: pathname of .bz2 file to be extracted.
-- @param outfile string or nil: pathname of output file to be produced.
-- If not given, name is derived from input file.
-- @return boolean: true on success; nil and error message on failure.
function tools.bunzip2(infile, outfile)
   return uncompress("bz2", "bunzip2", infile, outfile)
end

do
   local function rwx_to_octal(rwx)
      return (rwx:match "r" and 4 or 0)
         + (rwx:match "w" and 2 or 0)
         + (rwx:match "x" and 1 or 0)
   end
   local umask_cache
   function tools._unix_umask()
      if umask_cache then
         return umask_cache
      end
      local fd = assert(io.popen("umask -S"))
      local umask = assert(fd:read("*a"))
      fd:close()
      local u, g, o = umask:match("u=([rwx]*),g=([rwx]*),o=([rwx]*)")
      if not u then
         error("invalid umask result")
      end
      umask_cache = string.format("%d%d%d",
         7 - rwx_to_octal(u),
         7 - rwx_to_octal(g),
         7 - rwx_to_octal(o))
      return umask_cache
   end
end

--- Set permissions for file or directory
-- @param filename string: filename whose permissions are to be modified
-- @param mode string ("read" or "exec"): permissions to set
-- @param scope string ("user" or "all"): the user(s) to whom the permission applies
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message
function tools.set_permissions(filename, mode, scope)
   assert(filename and mode and scope)

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
   return fs.execute(vars.CHMOD, perms, filename)
end

function tools.browser(url)
   return fs.execute(cfg.web_browser, url)
end

-- Set access and modification times for a file.
-- @param filename File to set access and modification times for.
-- @param time may be a string or number containing the format returned
-- by os.time, or a table ready to be processed via os.time; if
-- nil, current time is assumed.
function tools.set_time(file, time)
   assert(time == nil or type(time) == "table" or type(time) == "number")
   file = dir.normalize(file)
   local flag = ""
   if type(time) == "number" then
      time = os.date("*t", time)
   end
   if type(time) == "table" then
      flag = ("-t %04d%02d%02d%02d%02d.%02d"):format(time.year, time.month, time.day, time.hour, time.min, time.sec)
   end
   return fs.execute(vars.TOUCH .. " " .. flag, file)
end

--- Create a temporary directory.
-- @param name_pattern string: name pattern to use for avoiding conflicts
-- when creating temporary directory.
-- @return string or (nil, string): name of temporary directory or (nil, error message) on failure.
function tools.make_temp_dir(name_pattern)
   assert(type(name_pattern) == "string")
   name_pattern = dir.normalize(name_pattern)

   local template = (os.getenv("TMPDIR") or "/tmp") .. "/luarocks_" .. name_pattern:gsub("/", "_") .. "-XXXXXX"
   local pipe = io.popen(vars.MKTEMP.." -d "..fs.Q(template))
   local dirname = pipe:read("*l")
   pipe:close()
   if dirname and dirname:match("^/") then
      return dirname
   end
   return nil, "Failed to create temporary directory "..tostring(dirname)
end

--- Test is file/directory exists
-- @param file string: filename to test
-- @return boolean: true if file exists, false otherwise.
function tools.exists(file)
   assert(file)
   return fs.execute(vars.TEST, "-e", file)
end

--- Test is pathname is a directory.
-- @param file string: pathname to test
-- @return boolean: true if it is a directory, false otherwise.
function tools.is_dir(file)
   assert(file)
   return fs.execute(vars.TEST, "-d", file)
end

--- Test is pathname is a regular file.
-- @param file string: pathname to test
-- @return boolean: true if it is a regular file, false otherwise.
function tools.is_file(file)
   assert(file)
   return fs.execute(vars.TEST, "-f", file)
end

function tools.current_user()
   local user = os.getenv("USER")
   if user then
      return user
   end
   local pd = io.popen("whoami", "r")
   if not pd then
      return ""
   end
   user = pd:read("*l")
   pd:close()
   return user
end

function tools.is_superuser()
   return fs.current_user() == "root"
end

function tools.lock_access(dirname, force)
   local ok, err = fs.make_dir(dirname)
   if not ok then
      return nil, err
   end

   local tempfile = dir.path(dirname, ".lock.tmp." .. tostring(math.random(100000000)))

   local fd, fderr = io.open(tempfile, "w")
   if not fd then
      return nil, "failed opening temp file " .. tempfile .. " for locking: " .. fderr
   end

   local ok, werr = fd:write("lock file for " .. dirname)
   if not ok then
      return nil, "failed writing temp file " .. tempfile .. " for locking: " .. werr
   end

   fd:close()

   local lockfile = dir.path(dirname, "lockfile.lfs")

   local force_flag = force and " -f" or ""

   if fs.execute(vars.LN .. force_flag, tempfile, lockfile) then
      return {
         tempfile = tempfile,
         lockfile = lockfile,
      }
   else
      return nil, "File exists" -- same message as luafilesystem
   end
end

function tools.unlock_access(lock)
   os.remove(lock.lockfile)
   os.remove(lock.tempfile)
end

return tools
