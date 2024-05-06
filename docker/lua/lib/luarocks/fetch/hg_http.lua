
--- Fetch back-end for retrieving sources from hg repositories
-- that use http:// transport. For example, for fetching a repository
-- that requires the following command line:
-- `hg clone http://example.com/foo`
-- you can use this in the rockspec:
-- source = { url = "hg+http://example.com/foo" }
local hg_http = {}

local hg = require("luarocks.fetch.hg")

--- Download sources for building a rock, using hg over http.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Unused in this module (required for API purposes.)
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function hg_http.get_sources(rockspec, extract, dest_dir)
   rockspec.source.url = rockspec.source.url:gsub("^hg.", "")
   return hg.get_sources(rockspec, extract, dest_dir)
end

return hg_http
