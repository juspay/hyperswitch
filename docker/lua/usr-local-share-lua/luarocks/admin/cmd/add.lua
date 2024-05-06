
--- Module implementing the luarocks-admin "add" command.
-- Adds a rock or rockspec to a rocks server.
local add = {}

local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")
local dir = require("luarocks.dir")
local writer = require("luarocks.manif.writer")
local fs = require("luarocks.fs")
local cache = require("luarocks.admin.cache")
local index = require("luarocks.admin.index")

function add.add_to_parser(parser)
   local cmd = parser:command("add", "Add a rock or rockspec to a rocks server.", util.see_also())

   cmd:argument("rock", "A local rockspec or rock file.")
      :args("+")

   cmd:option("--server", "The server to use. If not given, the default server "..
      "set in the upload_server variable from the configuration file is used instead.")
      :target("add_server")
   cmd:flag("--no-refresh", "Do not refresh the local cache prior to "..
      "generation of the updated manifest.")
   cmd:flag("--index", "Produce an index.html file for the manifest. This "..
      "flag is automatically set if an index.html file already exists.")
end

local function zip_manifests()
   for ver in util.lua_versions() do
      local file = "manifest-"..ver
      local zip = file..".zip"
      fs.delete(dir.path(fs.current_dir(), zip))
      fs.zip(zip, file)
   end
end

local function add_files_to_server(refresh, rockfiles, server, upload_server, do_index)
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

   if not login_url then
      login_url = protocol.."://"..server_path
   end

   local ok, err = fs.change_dir(at)
   if not ok then return nil, err end

   local files = {}
   for _, rockfile in ipairs(rockfiles) do
      if fs.exists(rockfile) then
         util.printout("Copying file "..rockfile.." to "..local_cache.."...")
         local absolute = fs.absolute_name(rockfile)
         fs.copy(absolute, local_cache, "read")
         table.insert(files, dir.base_name(absolute))
      else
         util.printerr("File "..rockfile.." not found")
      end
   end
   if #files == 0 then
      return nil, "No files found"
   end

   local ok, err = fs.change_dir(local_cache)
   if not ok then return nil, err end

   util.printout("Updating manifest...")
   writer.make_manifest(local_cache, "one", true)

   zip_manifests()

   if fs.exists("index.html") then
      do_index = true
   end

   if do_index then
      util.printout("Updating index.html...")
      index.make_index(local_cache)
   end

   local login_info = ""
   if user then login_info = " -u "..user end
   if password then login_info = login_info..":"..password end
   if not login_url:match("/$") then
      login_url = login_url .. "/"
   end

   if do_index then
      table.insert(files, "index.html")
   end
   table.insert(files, "manifest")
   for ver in util.lua_versions() do
      table.insert(files, "manifest-"..ver)
      table.insert(files, "manifest-"..ver..".zip")
   end

   -- TODO abstract away explicit 'curl' call

   local cmd
   if protocol == "rsync" then
      local srv, path = server_path:match("([^/]+)(/.+)")
      cmd = cfg.variables.RSYNC.." "..cfg.variables.RSYNCFLAGS.." -e ssh "..local_cache.."/ "..user.."@"..srv..":"..path.."/"
   elseif protocol == "file" then
      return fs.copy_contents(local_cache, server_path)
   elseif upload_server and upload_server.sftp then
      local part1, part2 = upload_server.sftp:match("^([^/]*)/(.*)$")
      cmd = cfg.variables.SCP.." "..table.concat(files, " ").." "..user.."@"..part1..":/"..part2
   else
      cmd = cfg.variables.CURL.." "..login_info.." -T '{"..table.concat(files, ",").."}' "..login_url
   end

   util.printout(cmd)
   return fs.execute(cmd)
end

function add.command(args)
   local server, server_table = cache.get_upload_server(args.add_server or args.server)
   if not server then return nil, server_table end
   return add_files_to_server(not args.no_refresh, args.rock, server, server_table, args.index)
end


return add
