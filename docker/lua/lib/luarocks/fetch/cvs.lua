
--- Fetch back-end for retrieving sources from CVS.
local cvs = {}

local unpack = unpack or table.unpack

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local util = require("luarocks.util")

--- Download sources for building a rock, using CVS.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Unused in this module (required for API purposes.)
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function cvs.get_sources(rockspec, extract, dest_dir)
   assert(rockspec:type() == "rockspec")
   assert(type(dest_dir) == "string" or not dest_dir)

   local cvs_cmd = rockspec.variables.CVS
   local ok, err_msg = fs.is_tool_available(cvs_cmd, "CVS")
   if not ok then
      return nil, err_msg
   end

   local name_version = rockspec.name .. "-" .. rockspec.version
   local module = rockspec.source.module or dir.base_name(rockspec.source.url)
   local command = {cvs_cmd, "-d"..rockspec.source.pathname, "export", module}
   if rockspec.source.tag then
      table.insert(command, 4, "-r")
      table.insert(command, 5, rockspec.source.tag)
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
   local ok, err = fs.change_dir(store_dir)
   if not ok then return nil, err end
   if not fs.execute(unpack(command)) then
      return nil, "Failed fetching files from CVS."
   end
   fs.pop_dir()
   return module, store_dir
end


return cvs
