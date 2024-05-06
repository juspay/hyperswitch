--- Linux-specific implementation of filesystem and platform abstractions.
local linux = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")

function linux.is_dir(file)
   file = fs.absolute_name(file)
   file = dir.normalize(file) .. "/."
   local fd, _, code = io.open(file, "r")
   if code == 2 then -- "No such file or directory"
      return false
   end
   if code == 20 then -- "Not a directory", regardless of permissions
      return false
   end
   if code == 13 then -- "Permission denied", but is a directory
      return true
   end
   if fd then
      local _, _, ecode = fd:read(1)
      fd:close()
      if ecode == 21 then -- "Is a directory"
         return true
      end
   end
   return false
end

function linux.is_file(file)
   file = fs.absolute_name(file)
   if fs.is_dir(file) then
      return false
   end
   file = dir.normalize(file)
   local fd, _, code = io.open(file, "r")
   if code == 2 then -- "No such file or directory"
      return false
   end
   if code == 13 then -- "Permission denied", but it exists
      return true
   end
   if fd then
      fd:close()
      return true
   end
   return false
end

return linux
