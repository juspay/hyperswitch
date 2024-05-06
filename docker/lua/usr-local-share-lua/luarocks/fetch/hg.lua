
--- Fetch back-end for retrieving sources from HG.
local hg = {}

local unpack = unpack or table.unpack

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local util = require("luarocks.util")

--- Download sources for building a rock, using hg.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Unused in this module (required for API purposes.)
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function hg.get_sources(rockspec, extract, dest_dir)
   assert(rockspec:type() == "rockspec")
   assert(type(dest_dir) == "string" or not dest_dir)

   local hg_cmd = rockspec.variables.HG
   local ok, err_msg = fs.is_tool_available(hg_cmd, "Mercurial")
   if not ok then
      return nil, err_msg
   end

   local name_version = rockspec.name .. "-" .. rockspec.version
   -- Strip off special hg:// protocol type
   local url = rockspec.source.url:gsub("^hg://", "")

   local module = dir.base_name(url)

   local command = {hg_cmd, "clone", url, module}
   local tag_or_branch = rockspec.source.tag or rockspec.source.branch
   if tag_or_branch then
      command = {hg_cmd, "clone", "--rev", tag_or_branch, url, module}
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
      return nil, "Failed cloning hg repository."
   end
   ok, err = fs.change_dir(module)
   if not ok then return nil, err end

   fs.delete(dir.path(store_dir, module, ".hg"))
   fs.delete(dir.path(store_dir, module, ".hgignore"))
   fs.pop_dir()
   fs.pop_dir()
   return module, store_dir
end


return hg
