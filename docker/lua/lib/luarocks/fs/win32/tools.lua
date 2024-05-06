
--- fs operations implemented with third-party tools for Windows platform abstractions.
-- Download http://unxutils.sourceforge.net/ for Windows GNU utilities
-- used by this module.
local tools = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local cfg = require("luarocks.core.cfg")

local vars = setmetatable({}, { __index = function(_,k) return cfg.variables[k] end })

local dir_sep = package.config:sub(1, 1)

--- Adds prefix to command to make it run from a directory.
-- @param directory string: Path to a directory.
-- @param cmd string: A command-line string.
-- @param exit_on_error bool: Exits immediately if entering the directory failed.
-- @return string: The command-line with prefix.
function tools.command_at(directory, cmd, exit_on_error)
   local drive = directory:match("^([A-Za-z]:)")
   local op = " & "
   if exit_on_error then
      op = " && "
   end
   local cmd_prefixed = "cd " .. fs.Q(directory) .. op .. cmd
   if drive then
      cmd_prefixed = drive .. " & " .. cmd_prefixed
   end
   return cmd_prefixed
end

--- Create a directory if it does not already exist.
-- If any of the higher levels in the path name does not exist
-- too, they are created as well.
-- @param directory string: pathname of directory to create.
-- @return boolean: true on success, false on failure.
function tools.make_dir(directory)
   assert(directory)
   directory = dir.normalize(directory)
   fs.execute_quiet(vars.MKDIR, directory)
   if not fs.is_dir(directory) then
      return false, "failed making directory "..directory
   end
   return true
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
   while true do
      fs.execute_quiet(vars.RMDIR, directory)
      local parent = dir.dir_name(directory)
      if parent ~= directory then
         directory = parent
      else
         break
      end
   end
end

--- Copy a file.
-- @param src string: Pathname of source
-- @param dest string: Pathname of destination
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message.
function tools.copy(src, dest)
   assert(src and dest)
   if dest:match("[/\\]$") then dest = dest:sub(1, -2) end
   local ok = fs.execute(vars.CP, src, dest)
   if ok then
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
   if fs.make_dir(dest) and fs.execute_quiet(vars.CP, "-dR", src.."\\*.*", dest) then
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
   assert(arg:match("^[a-zA-Z]?:?[\\/]"))
   fs.execute_quiet("if exist "..fs.Q(arg.."\\*").." ( RMDIR /S /Q "..fs.Q(arg).." ) else ( DEL /Q /F "..fs.Q(arg).." )")
end

--- Recursively scan the contents of a directory.
-- @param at string or nil: directory to scan (will be the current
-- directory if none is given).
-- @return table: an array of strings with the filenames representing
-- the contents of a directory. Paths are returned with forward slashes.
function tools.find(at)
   assert(type(at) == "string" or not at)
   if not at then
      at = fs.current_dir()
   end
   if not fs.is_dir(at) then
      return {}
   end
   local result = {}
   local pipe = io.popen(fs.command_at(at, fs.quiet_stderr(vars.FIND), true))
   for file in pipe:lines() do
      -- Windows find is a bit different
      local first_two = file:sub(1,2)
      if first_two == ".\\" or first_two == "./" then file=file:sub(3) end
      if file ~= "." then
         table.insert(result, (file:gsub("[\\/]", dir_sep)))
      end
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
   if fs.execute_quiet(vars.SEVENZ.." -aoa a -tzip", zipfile, ...) then
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
   if fs.execute_quiet(vars.SEVENZ.." -aoa x", zipfile) then
      return true
   else
      return nil, "failed extracting " .. zipfile
   end
end

local function sevenz(default_ext, infile, outfile)
   assert(type(infile) == "string")
   assert(outfile == nil or type(outfile) == "string")

   local dropext = infile:gsub("%."..default_ext.."$", "")
   local outdir = dir.dir_name(dropext)

   infile = fs.absolute_name(infile)

   local cmdline = vars.SEVENZ.." -aoa -t* -o"..fs.Q(outdir).." x "..fs.Q(infile)
   local ok, err = fs.execute_quiet(cmdline)
   if not ok then
      return nil, "failed extracting " .. infile
   end

   if outfile then
      outfile = fs.absolute_name(outfile)
      dropext = fs.absolute_name(dropext)
      ok, err = os.rename(dropext, outfile)
      if not ok then
         return nil, "failed creating new file " .. outfile
      end
   end

   return true
end

--- Uncompresses a .gz file.
-- @param infile string: pathname of .gz file to be extracted.
-- @param outfile string or nil: pathname of output file to be produced.
-- If not given, name is derived from input file.
-- @return boolean: true on success; nil and error message on failure.
function tools.gunzip(infile, outfile)
   return sevenz("gz", infile, outfile)
end

--- Uncompresses a .bz2 file.
-- @param infile string: pathname of .bz2 file to be extracted.
-- @param outfile string or nil: pathname of output file to be produced.
-- If not given, name is derived from input file.
-- @return boolean: true on success; nil and error message on failure.
function tools.bunzip2(infile, outfile)
   return sevenz("bz2", infile, outfile)
end

--- Helper function for fs.set_permissions
-- @return table: an array of all system users
local function get_system_users()
   local exclude = {
      [""]              = true,
      ["Name"]          = true,
      ["\128\164\172\168\173\168\225\226\224\160\226\174\224"] = true, -- Administrator in cp866
      ["Administrator"] = true,
   }
   local result = {}
   local fd = assert(io.popen("wmic UserAccount get name"))
   for user in fd:lines() do
      user = user:gsub("%s+$", "")
      if not exclude[user] then
         table.insert(result, user)
      end
   end
   return result
end

--- Set permissions for file or directory
-- @param filename string: filename whose permissions are to be modified
-- @param mode string ("read" or "exec"): permission to set
-- @param scope string ("user" or "all"): the user(s) to whom the permission applies
-- @return boolean or (boolean, string): true on success, false on failure,
-- plus an error message
function tools.set_permissions(filename, mode, scope)
   assert(filename and mode and scope)

   if scope == "user" then
      local perms
      if mode == "read" then
         perms = "(R,W,M)"
      elseif mode == "exec" then
         perms = "(F)"
      end

      local ok
      -- Take ownership of the given file
      ok = fs.execute_quiet("takeown /f " .. fs.Q(filename))
      if not ok then
         return false, "Could not take ownership of the given file"
      end
      local username = os.getenv('USERNAME')
      -- Grant the current user the proper rights
      ok = fs.execute_quiet(vars.ICACLS .. " " .. fs.Q(filename) .. " /inheritance:d /grant:r " .. fs.Q(username) .. ":" .. perms)
      if not ok then
         return false, "Failed setting permission " .. mode .. " for " .. scope
      end
      -- Finally, remove all the other users from the ACL in order to deny them access to the file
      for _, user in pairs(get_system_users()) do
         if username ~= user then
            local ok = fs.execute_quiet(vars.ICACLS .. " " .. fs.Q(filename) .. " /remove " .. fs.Q(user))
            if not ok then
               return false, "Failed setting permission " .. mode .. " for " .. scope
            end
         end
      end
   elseif scope == "all" then
      local my_perms, others_perms
      if mode == "read" then
         my_perms = "(R,W,M)"
         others_perms = "(R)"
      elseif mode == "exec" then
         my_perms = "(F)"
         others_perms = "(RX)"
      end

      local ok
      -- Grant permissions available to all users
      ok = fs.execute_quiet(vars.ICACLS .. " " .. fs.Q(filename) .. " /inheritance:d /grant:r *S-1-1-0:" .. others_perms)
      if not ok then
         return false, "Failed setting permission " .. mode .. " for " .. scope
      end

      -- Grant permissions available only to the current user
      ok = fs.execute_quiet(vars.ICACLS .. " " .. fs.Q(filename) .. " /inheritance:d /grant \"%USERNAME%\":" .. my_perms)

      -- This may not be necessary if the above syntax is correct,
      -- but I couldn't really test the extra quotes above, so if that
      -- fails we try again with the syntax used in previous releases
      -- just to be on the safe side
      if not ok then
         ok = fs.execute_quiet(vars.ICACLS .. " " .. fs.Q(filename) .. " /inheritance:d /grant %USERNAME%:" .. my_perms)
      end

      if not ok then
         return false, "Failed setting permission " .. mode .. " for " .. scope
      end
   end

   return true
end

function tools.browser(url)
   return fs.execute(cfg.web_browser..' "Starting docs..." '..fs.Q(url))
end

-- Set access and modification times for a file.
-- @param filename File to set access and modification times for.
-- @param time may be a string or number containing the format returned
-- by os.time, or a table ready to be processed via os.time; if
-- nil, current time is assumed.
function tools.set_time(filename, time)
   return true -- FIXME
end

function tools.lock_access(dirname)
   -- NYI
   return {}
end

function tools.unlock_access(lock)
   -- NYI
end

return tools
