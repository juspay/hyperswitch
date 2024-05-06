
--- Module handling the LuaRocks local cache.
-- Adds a rock or rockspec to a rocks server.
local cache = {}

local fs = require("luarocks.fs")
local cfg = require("luarocks.core.cfg")
local dir = require("luarocks.dir")
local util = require("luarocks.util")

function cache.get_upload_server(server)
   if not server then server = cfg.upload_server end
   if not server then
      return nil, "No server specified and no default configured with upload_server."
   end
   return server, cfg.upload_servers and cfg.upload_servers[server]
end

function cache.get_server_urls(server, upload_server)
   local download_url = server
   local login_url = nil
   if upload_server then
      if upload_server.rsync then download_url = "rsync://"..upload_server.rsync
      elseif upload_server.http then download_url = "http://"..upload_server.http
      elseif upload_server.ftp then download_url = "ftp://"..upload_server.ftp
      end

      if upload_server.ftp then login_url = "ftp://"..upload_server.ftp
      elseif upload_server.sftp then login_url = "sftp://"..upload_server.sftp
      end
   end
   return download_url, login_url
end

function cache.split_server_url(url, user, password)
   local protocol, server_path = dir.split_url(url)
   if protocol == "file" then
      server_path = fs.absolute_name(server_path)
   elseif server_path:match("@") then
      local credentials
      credentials, server_path = server_path:match("([^@]*)@(.*)")
      if credentials:match(":") then
         user, password = credentials:match("([^:]*):(.*)")
      else
         user = credentials
      end
   end
   local local_cache = dir.path(cfg.local_cache, (server_path:gsub("[\\/]", "_")))
   return local_cache, protocol, server_path, user, password
end

local function download_cache(protocol, server_path, user, password)
   os.remove("index.html")
   -- TODO abstract away explicit 'wget' call
   if protocol == "rsync" then
      local srv, path = server_path:match("([^/]+)(/.+)")
      return fs.execute(cfg.variables.RSYNC.." "..cfg.variables.RSYNCFLAGS.." -e ssh "..user.."@"..srv..":"..path.."/ ./")
   elseif protocol == "file" then
      return fs.copy_contents(server_path, ".")
   else
      local login_info = ""
      if user then login_info = " --user="..user end
      if password then login_info = login_info .. " --password="..password end
      return fs.execute(cfg.variables.WGET.." --no-cache -q -m -np -nd "..protocol.."://"..server_path..login_info)
   end
end

function cache.refresh_local_cache(url, given_user, given_password)
   local local_cache, protocol, server_path, user, password = cache.split_server_url(url, given_user, given_password)

   local ok, err = fs.make_dir(local_cache)
   if not ok then
      return nil, "Failed creating local cache dir: "..err
   end

   fs.change_dir(local_cache)

   util.printout("Refreshing cache "..local_cache.."...")

   ok = download_cache(protocol, server_path, user, password)
   if not ok then
      return nil, "Failed downloading cache."
   end

   return local_cache, protocol, server_path, user, password
end

return cache
