
--- Fetch back-end for retrieving sources from hg repositories
-- that use ssh:// transport. For example, for fetching a repository
-- that requires the following command line:
-- `hg clone ssh://example.com/foo`
-- you can use this in the rockspec:
-- source = { url = "hg+ssh://example.com/foo" }
return require "luarocks.fetch.hg_http"
