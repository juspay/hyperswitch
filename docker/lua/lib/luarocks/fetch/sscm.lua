
--- Fetch back-end for retrieving sources from Surround SCM Server
local sscm = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")

--- Download sources via Surround SCM Server for building a rock.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Unused in this module (required for API purposes.)
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function sscm.get_sources(rockspec, extract, dest_dir)
   assert(rockspec:type() == "rockspec")
   assert(type(dest_dir) == "string" or not dest_dir)

   local sscm_cmd = rockspec.variables.SSCM
   local module = rockspec.source.module or dir.base_name(rockspec.source.url)
   local branch, repository = string.match(rockspec.source.pathname, "^([^/]*)/(.*)")
   if not branch or not repository then
      return nil, "Error retrieving branch and repository from rockspec."
   end
   -- Search for working directory.
   local working_dir
   local tmp = io.popen(string.format(sscm_cmd..[[ property "/" -d -b%s -p%s]], branch, repository))
   for line in tmp:lines() do
      --%c because a chr(13) comes in the end.
      working_dir = string.match(line, "Working directory:[%s]*(.*)%c$")
      if working_dir then break end
   end
   tmp:close()
   if not working_dir then
      return nil, "Error retrieving working directory from SSCM."
   end
   if not fs.execute(sscm_cmd, "get", "*", "-e" , "-r", "-b"..branch, "-p"..repository, "-tmodify", "-wreplace") then
      return nil, "Failed fetching files from SSCM."
   end
   -- FIXME: This function does not honor the dest_dir parameter.
   return module, working_dir
end

return sscm
