
--- Module implementing the luarocks-admin "remove" command.
-- Removes a rock or rockspec from a rocks server.
local admin_remove = {}

local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")
local dir = require("luarocks.dir")
local writer = require("luarocks.manif.writer")
local fs = require("luarocks.fs")
local cache = require("luarocks.admin.cache")
local index = require("luarocks.admin.index")

function admin_remove.add_to_parser(parser)
   local cmd = parser:command("remove", "Remove a rock or rockspec from a rocks server.", util.see_also())

   cmd:argument("rock", "A local rockspec or rock file.")
      :args("+")

   cmd:option("--server", "The server to use. If not given, the default server "..
      "set in the upload_server variable from the configuration file is used instead.")
   cmd:flag("--no-refresh", "Do not refresh the local cache prior to "..
      "generation of the updated manifest.")
end

local function remove_files_from_server(refresh, rockfiles, server, upload_server)
   assert(type(refresh) == "boolean" or not refresh)
   assert(type(rockfiles) == "table")
   assert(type(server) == "string")
   assert(type(upload_server) == "table" or not upload_server)

   local download_url, login_url = cache.get_server_urls(server, upload_server)
   local at = fs.current_dir()
   local refresh_fn = refresh and cache.refresh_local_cache or cache.split_server_url

   local local_cache, protocol, server_path, user, password = refresh_fn(download_url, cfg.upload_user, cfg.upload_password)
   if not local_cache then
      return nil, protocol
   end

   local ok, err = fs.change_dir(at)
   if not ok then return nil, err end

   local nr_files = 0
   for _, rockfile in ipairs(rockfiles) do
      local basename = dir.base_name(rockfile)
      local file = dir.path(local_cache, basename)
      util.printout("Removing file "..file.."...")
      fs.delete(file)
      if not fs.exists(file) then
         nr_files = nr_files + 1
      else
         util.printerr("Failed removing "..file)
      end
   end
   if nr_files == 0 then
      return nil, "No files removed."
   end

   local ok, err = fs.change_dir(local_cache)
   if not ok then return nil, err end

   util.printout("Updating manifest...")
   writer.make_manifest(local_cache, "one", true)
   util.printout("Updating index.html...")
   index.make_index(local_cache)

   if protocol == "file" then
       local cmd = cfg.variables.RSYNC.." "..cfg.variables.RSYNCFLAGS.." --delete "..local_cache.."/ ".. server_path.."/"
       util.printout(cmd)
       fs.execute(cmd)
       return true
   end

   if protocol ~= "rsync" then
      return nil, "This command requires 'rsync', check your configuration."
   end

   local srv, path = server_path:match("([^/]+)(/.+)")
   local cmd = cfg.variables.RSYNC.." "..cfg.variables.RSYNCFLAGS.." --delete -e ssh "..local_cache.."/ "..user.."@"..srv..":"..path.."/"

   util.printout(cmd)
   fs.execute(cmd)

   return true
end

function admin_remove.command(args)
   local server, server_table = cache.get_upload_server(args.server)
   if not server then return nil, server_table end
   return remove_files_from_server(not args.no_refresh, args.rock, server, server_table)
end


return admin_remove
