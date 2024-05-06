
--- Fetch back-end for retrieving sources from hg repositories
-- that use https:// transport. For example, for fetching a repository
-- that requires the following command line:
-- `hg clone https://example.com/foo`
-- you can use this in the rockspec:
-- source = { url = "hg+https://example.com/foo" }
return require "luarocks.fetch.hg_http"
