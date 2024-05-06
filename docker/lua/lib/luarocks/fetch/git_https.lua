--- Fetch back-end for retrieving sources from Git repositories
-- that use https:// transport. For example, for fetching a repository
-- that requires the following command line:
-- `git clone https://example.com/foo.git`
-- you can use this in the rockspec:
-- source = { url = "git+https://example.com/foo.git" }
return require "luarocks.fetch.git_http"
