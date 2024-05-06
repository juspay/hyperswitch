
--- Fetch back-end for retrieving sources from Git repositories
-- that use http:// transport. For example, for fetching a repository
-- that requires the following command line:
-- `git clone http://example.com/foo.git`
-- you can use this in the rockspec:
-- source = { url = "git+http://example.com/foo.git" }
-- Prefer using the normal git:// fetch mode as it is more widely
-- available in older versions of LuaRocks.
local git_http = {}

local git = require("luarocks.fetch.git")

--- Fetch sources for building a rock from a local Git repository.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Unused in this module (required for API purposes.)
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function git_http.get_sources(rockspec, extract, dest_dir)
   rockspec.source.url = rockspec.source.url:gsub("^git.", "")
   return git.get_sources(rockspec, extract, dest_dir, "--")
end

return git_http
