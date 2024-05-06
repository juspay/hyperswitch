
--- Functions related to fetching and loading local and remote files.
local fetch = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local rockspecs = require("luarocks.rockspecs")
local signing = require("luarocks.signing")
local persist = require("luarocks.persist")
local util = require("luarocks.util")
local cfg = require("luarocks.core.cfg")


--- Fetch a local or remote file, using a local cache directory.
-- Make a remote or local URL/pathname local, fetching the file if necessary.
-- Other "fetch" and "load" functions use this function to obtain files.
-- If a local pathname is given, it is returned as a result.
-- @param url string: a local pathname or a remote URL.
-- @param mirroring string: mirroring mode.
-- If set to "no_mirror", then rocks_servers mirror configuration is not used.
-- @return (string, nil, nil, boolean) or (nil, string, [string]):
-- in case of success:
-- * the absolute local pathname for the fetched file
-- * nil
-- * nil
-- * `true` if the file was fetched from cache
-- in case of failure:
-- * nil
-- * an error message
-- * an optional error code.
function fetch.fetch_caching(url, mirroring)
   local repo_url, filename = url:match("^(.*)/([^/]+)$")
   local name = repo_url:gsub("[/:]","_")
   local cache_dir = dir.path(cfg.local_cache, name)
   local ok = fs.make_dir(cache_dir)

   local cachefile = dir.path(cache_dir, filename)
   local checkfile = cachefile .. ".check"

   if (fs.file_age(checkfile) < 10 or
      cfg.aggressive_cache and (not name:match("^manifest"))) and fs.exists(cachefile)
   then
      return cachefile, nil, nil, true
   end

   local lock, errlock
   if ok then
      lock, errlock = fs.lock_access(cache_dir)
   end

   if not (ok and lock) then
      cfg.local_cache = fs.make_temp_dir("local_cache")
      if not cfg.local_cache then
         return nil, "Failed creating temporary local_cache directory"
      end
      cache_dir = dir.path(cfg.local_cache, name)
      ok = fs.make_dir(cache_dir)
      if not ok then
         return nil, "Failed creating temporary cache directory "..cache_dir
      end
      lock = fs.lock_access(cache_dir)
   end

   local file, err, errcode, from_cache = fetch.fetch_url(url, cachefile, true, mirroring)

   local fd = io.open(checkfile, "wb")
   fd:write("!")
   fd:close()

   fs.unlock_access(lock)
   if not file then
      return nil, err or "Failed downloading "..url, errcode
   end
   return file, nil, nil, from_cache
end

local function ensure_trailing_slash(url)
   return (url:gsub("/*$", "/"))
end

local function is_url_relative_to_rocks_servers(url, servers)
   for _, item in ipairs(servers) do
      if type(item) == "table" then
         for i, s in ipairs(item) do
            local base = ensure_trailing_slash(s)
            if string.find(url, base, 1, true) == 1 then
               return i, url:sub(#base + 1), item
            end
         end
      end
   end
end

local function download_with_mirrors(url, filename, cache, servers)
   local idx, rest, mirrors = is_url_relative_to_rocks_servers(url, servers)

   if not idx then
      -- URL is not from a rock server
      return fs.download(url, filename, cache)
   end

   -- URL is from a rock server: try to download it falling back to mirrors.
   local err = "\n"
   for i = idx, #mirrors do
      local try_url = ensure_trailing_slash(mirrors[i]) .. rest
      if i > idx then
         util.warning("Failed downloading. Attempting mirror at " .. try_url)
      end
      local ok, name, from_cache = fs.download(try_url, filename, cache)
      if ok then
         return ok, name, from_cache
      else
         err = err .. name .. "\n"
      end
   end

   return nil, err, "network"
end

--- Fetch a local or remote file.
-- Make a remote or local URL/pathname local, fetching the file if necessary.
-- Other "fetch" and "load" functions use this function to obtain files.
-- If a local pathname is given, it is returned as a result.
-- @param url string: a local pathname or a remote URL.
-- @param filename string or nil: this function attempts to detect the
-- resulting local filename of the remote file as the basename of the URL;
-- if that is not correct (due to a redirection, for example), the local
-- filename can be given explicitly as this second argument.
-- @param cache boolean: compare remote timestamps via HTTP HEAD prior to
-- re-downloading the file.
-- @param mirroring string: mirroring mode.
-- If set to "no_mirror", then rocks_servers mirror configuration is not used.
-- @return (string, nil, nil, boolean) or (nil, string, [string]):
-- in case of success:
-- * the absolute local pathname for the fetched file
-- * nil
-- * nil
-- * `true` if the file was fetched from cache
-- in case of failure:
-- * nil
-- * an error message
-- * an optional error code.
function fetch.fetch_url(url, filename, cache, mirroring)
   assert(type(url) == "string")
   assert(type(filename) == "string" or not filename)

   local protocol, pathname = dir.split_url(url)
   if protocol == "file" then
      local fullname = fs.absolute_name(pathname)
      if not fs.exists(fullname) then
         local hint = (not pathname:match("^/"))
                      and (" - note that given path in rockspec is not absolute: " .. url)
                      or  ""
         return nil, "Local file not found: " .. fullname .. hint
      end
      filename = filename or dir.base_name(pathname)
      local dstname = fs.absolute_name(dir.path(".", filename))
      local ok, err
      if fullname == dstname then
         ok = true
      else
         ok, err = fs.copy(fullname, dstname)
      end
      if ok then
         return dstname
      else
         return nil, "Failed copying local file " .. fullname .. " to " .. dstname .. ": " .. err
      end
   elseif dir.is_basic_protocol(protocol) then
      local ok, name, from_cache
      if mirroring ~= "no_mirror" then
         ok, name, from_cache = download_with_mirrors(url, filename, cache, cfg.rocks_servers)
      else
         ok, name, from_cache = fs.download(url, filename, cache)
      end
      if not ok then
         return nil, "Failed downloading "..url..(name and " - "..name or ""), from_cache
      end
      return name, nil, nil, from_cache
   else
      return nil, "Unsupported protocol "..protocol
   end
end

--- For remote URLs, create a temporary directory and download URL inside it.
-- This temporary directory will be deleted on program termination.
-- For local URLs, just return the local pathname and its directory.
-- @param url string: URL to be downloaded
-- @param tmpname string: name pattern to use for avoiding conflicts
-- when creating temporary directory.
-- @param filename string or nil: local filename of URL to be downloaded,
-- in case it can't be inferred from the URL.
-- @return (string, string) or (nil, string, [string]): absolute local pathname of
-- the fetched file and temporary directory name; or nil and an error message
-- followed by an optional error code
function fetch.fetch_url_at_temp_dir(url, tmpname, filename, cache)
   assert(type(url) == "string")
   assert(type(tmpname) == "string")
   assert(type(filename) == "string" or not filename)
   filename = filename or dir.base_name(url)

   local protocol, pathname = dir.split_url(url)
   if protocol == "file" then
      if fs.exists(pathname) then
         return pathname, dir.dir_name(fs.absolute_name(pathname))
      else
         return nil, "File not found: "..pathname
      end
   else
      local temp_dir, err = fs.make_temp_dir(tmpname)
      if not temp_dir then
         return nil, "Failed creating temporary directory "..tmpname..": "..err
      end
      util.schedule_function(fs.delete, temp_dir)
      local ok, err = fs.change_dir(temp_dir)
      if not ok then return nil, err end

      local file, err, errcode

      if cache then
         local cachefile
         cachefile, err, errcode = fetch.fetch_caching(url)

         if cachefile then
            file = dir.path(temp_dir, filename)
            fs.copy(cachefile, file)
         end
      end

      if not file then
         file, err, errcode = fetch.fetch_url(url, filename, cache)
      end

      fs.pop_dir()
      if not file then
         return nil, "Error fetching file: "..err, errcode
      end

      return file, temp_dir
   end
end

-- Determine base directory of a fetched URL by extracting its
-- archive and looking for a directory in the root.
-- @param file string: absolute local pathname of the fetched file
-- @param temp_dir string: temporary directory in which URL was fetched.
-- @param src_url string: URL to use when inferring base directory.
-- @param src_dir string or nil: expected base directory (inferred
-- from src_url if not given).
-- @return (string, string) or (string, nil) or (nil, string):
-- The inferred base directory and the one actually found (which may
-- be nil if not found), or nil followed by an error message.
-- The inferred dir is returned first to avoid confusion with errors,
-- because it is never nil.
function fetch.find_base_dir(file, temp_dir, src_url, src_dir)
   local ok, err = fs.change_dir(temp_dir)
   if not ok then return nil, err end
   fs.unpack_archive(file)

   if not src_dir then
      local rockspec = {
         source = {
            file = file,
            dir = src_dir,
            url = src_url,
         }
      }
      ok, err = fetch.find_rockspec_source_dir(rockspec, ".")
      if ok then
         src_dir = rockspec.source.dir
      end
   end

   local inferred_dir = src_dir or dir.deduce_base_dir(src_url)
   local found_dir = nil
   if fs.exists(inferred_dir) then
      found_dir = inferred_dir
   else
      util.printerr("Directory "..inferred_dir.." not found")
      local files = fs.list_dir()
      if files then
         table.sort(files)
         for i,filename in ipairs(files) do
            if fs.is_dir(filename) then
               util.printerr("Found "..filename)
               found_dir = filename
               break
            end
         end
      end
   end
   fs.pop_dir()
   return inferred_dir, found_dir
end

local function fetch_and_verify_signature_for(url, filename, tmpdir)
   local sig_url = signing.signature_url(url)
   local sig_file, err, errcode = fetch.fetch_url_at_temp_dir(sig_url, tmpdir)
   if not sig_file then
      return nil, "Could not fetch signature file for verification: " .. err, errcode
   end

   local ok, err = signing.verify_signature(filename, sig_file)
   if not ok then
      return nil, "Failed signature verification: " .. err
   end

   return fs.absolute_name(sig_file)
end

--- Obtain a rock and unpack it.
-- If a directory is not given, a temporary directory will be created,
-- which will be deleted on program termination.
-- @param rock_file string: URL or filename of the rock.
-- @param dest string or nil: if given, directory will be used as
-- a permanent destination.
-- @param verify boolean: if true, download and verify signature for rockspec
-- @return string or (nil, string, [string]): the directory containing the contents
-- of the unpacked rock.
function fetch.fetch_and_unpack_rock(url, dest, verify)
   assert(type(url) == "string")
   assert(type(dest) == "string" or not dest)

   local name = dir.base_name(url):match("(.*)%.[^.]*%.rock")
   local tmpname = "luarocks-rock-" .. name

   local rock_file, err, errcode = fetch.fetch_url_at_temp_dir(url, tmpname, nil, true)
   if not rock_file then
      return nil, "Could not fetch rock file: " .. err, errcode
   end

   local sig_file
   if verify then
      sig_file, err = fetch_and_verify_signature_for(url, rock_file, tmpname)
      if err then
         return nil, err
      end
   end

   rock_file = fs.absolute_name(rock_file)

   local unpack_dir
   if dest then
      unpack_dir = dest
      local ok, err = fs.make_dir(unpack_dir)
      if not ok then
         return nil, "Failed unpacking rock file: " .. err
      end
   else
      unpack_dir, err = fs.make_temp_dir(name)
      if not unpack_dir then
         return nil, "Failed creating temporary dir: " .. err
      end
   end
   if not dest then
      util.schedule_function(fs.delete, unpack_dir)
   end
   local ok, err = fs.change_dir(unpack_dir)
   if not ok then return nil, err end
   ok, err = fs.unzip(rock_file)
   if not ok then
      return nil, "Failed unpacking rock file: " .. rock_file .. ": " .. err
   end
   if sig_file then
      ok, err = fs.copy(sig_file, ".")
      if not ok then
         return nil, "Failed copying signature file"
      end
   end
   fs.pop_dir()
   return unpack_dir
end

--- Back-end function that actually loads the local rockspec.
-- Performs some validation and postprocessing of the rockspec contents.
-- @param rel_filename string: The local filename of the rockspec file.
-- @param quick boolean: if true, skips some steps when loading
-- rockspec.
-- @return table or (nil, string): A table representing the rockspec
-- or nil followed by an error message.
function fetch.load_local_rockspec(rel_filename, quick)
   assert(type(rel_filename) == "string")
   local abs_filename = fs.absolute_name(rel_filename)

   local basename = dir.base_name(abs_filename)
   if basename ~= "rockspec" then
      if not basename:match("(.*)%-[^-]*%-[0-9]*") then
         return nil, "Expected filename in format 'name-version-revision.rockspec'."
      end
   end

   local tbl, err = persist.load_into_table(abs_filename)
   if not tbl then
      return nil, "Could not load rockspec file "..abs_filename.." ("..err..")"
   end

   local rockspec, err = rockspecs.from_persisted_table(abs_filename, tbl, err, quick)
   if not rockspec then
      return nil, abs_filename .. ": " .. err
   end

   local name_version = rockspec.package:lower() .. "-" .. rockspec.version
   if basename ~= "rockspec" and basename ~= name_version .. ".rockspec" then
      return nil, "Inconsistency between rockspec filename ("..basename..") and its contents ("..name_version..".rockspec)."
   end

   return rockspec
end

--- Load a local or remote rockspec into a table.
-- This is the entry point for the LuaRocks tools.
-- Only the LuaRocks runtime loader should use
-- load_local_rockspec directly.
-- @param filename string: Local or remote filename of a rockspec.
-- @param location string or nil: Where to download. If not given,
-- a temporary dir is created.
-- @param verify boolean: if true, download and verify signature for rockspec
-- @return table or (nil, string, [string]): A table representing the rockspec
-- or nil followed by an error message and optional error code.
function fetch.load_rockspec(url, location, verify)
   assert(type(url) == "string")

   local name
   local basename = dir.base_name(url)
   if basename == "rockspec" then
      name = "rockspec"
   else
      name = basename:match("(.*)%.rockspec")
      if not name then
         return nil, "Filename '"..url.."' does not look like a rockspec."
      end
   end

   local tmpname = "luarocks-rockspec-"..name
   local filename, err, errcode
   if location then
      local ok, err = fs.change_dir(location)
      if not ok then return nil, err end
      filename, err = fetch.fetch_url(url)
      fs.pop_dir()
   else
      filename, err, errcode = fetch.fetch_url_at_temp_dir(url, tmpname, nil, true)
   end
   if not filename then
      return nil, err, errcode
   end

   if verify then
      local _, err = fetch_and_verify_signature_for(url, filename, tmpname)
      if err then
         return nil, err
      end
   end

   return fetch.load_local_rockspec(filename)
end

--- Download sources for building a rock using the basic URL downloader.
-- @param rockspec table: The rockspec table
-- @param extract boolean: Whether to extract the sources from
-- the fetched source tarball or not.
-- @param dest_dir string or nil: If set, will extract to the given directory;
-- if not given, will extract to a temporary directory.
-- @return (string, string) or (nil, string, [string]): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message and optional error code.
function fetch.get_sources(rockspec, extract, dest_dir)
   assert(rockspec:type() == "rockspec")
   assert(type(extract) == "boolean")
   assert(type(dest_dir) == "string" or not dest_dir)

   local url = rockspec.source.url
   local name = rockspec.name.."-"..rockspec.version
   local filename = rockspec.source.file
   local source_file, store_dir
   local ok, err, errcode
   if dest_dir then
      ok, err = fs.change_dir(dest_dir)
      if not ok then return nil, err, "dest_dir" end
      source_file, err, errcode = fetch.fetch_url(url, filename)
      fs.pop_dir()
      store_dir = dest_dir
   else
      source_file, store_dir, errcode = fetch.fetch_url_at_temp_dir(url, "luarocks-source-"..name, filename)
   end
   if not source_file then
      return nil, err or store_dir, errcode
   end
   if rockspec.source.md5 then
      if not fs.check_md5(source_file, rockspec.source.md5) then
         return nil, "MD5 check for "..filename.." has failed.", "md5"
      end
   end
   if extract then
      local ok, err = fs.change_dir(store_dir)
      if not ok then return nil, err end
      ok, err = fs.unpack_archive(rockspec.source.file)
      if not ok then return nil, err end
      ok, err = fetch.find_rockspec_source_dir(rockspec, ".")
      if not ok then return nil, err end
      fs.pop_dir()
   end
   return source_file, store_dir
end

function fetch.find_rockspec_source_dir(rockspec, store_dir)
   local ok, err = fs.change_dir(store_dir)
   if not ok then return nil, err end

   local file_count, dir_count, found_dir = 0, 0, 0

   if rockspec.source.dir and fs.exists(rockspec.source.dir) then
      ok, err = true, nil
   elseif rockspec.source.file and rockspec.source.dir then
      ok, err = nil, "Directory "..rockspec.source.dir.." not found inside archive "..rockspec.source.file
   elseif not rockspec.source.dir_set then -- and rockspec:format_is_at_least("3.0") then

      local name = dir.base_name(rockspec.source.file or rockspec.source.url or "")

      if name:match("%.lua$") or name:match("%.c$") then
         if fs.is_file(name) then
            rockspec.source.dir = "."
            ok, err = true, nil
         end
      end

      if not rockspec.source.dir then
         for file in fs.dir() do
            file_count = file_count + 1
            if fs.is_dir(file) then
               dir_count = dir_count + 1
               found_dir = file
            end
         end

         if dir_count == 1 then
            rockspec.source.dir = found_dir
            ok, err = true, nil
         else
            ok, err = nil, "Could not determine source directory from rock contents (" .. tostring(file_count).." file(s), "..tostring(dir_count).." dir(s))"
         end
      end
   else
      ok, err = nil, "Could not determine source directory, please set source.dir in rockspec."
   end

   fs.pop_dir()

   assert(rockspec.source.dir or not ok)
   return ok, err
end

--- Download sources for building a rock, calling the appropriate protocol method.
-- @param rockspec table: The rockspec table
-- @param extract boolean: When downloading compressed formats, whether to extract
-- the sources from the fetched archive or not.
-- @param dest_dir string or nil: If set, will extract to the given directory.
-- if not given, will extract to a temporary directory.
-- @return (string, string) or (nil, string): The absolute pathname of
-- the fetched source tarball and the temporary directory created to
-- store it; or nil and an error message.
function fetch.fetch_sources(rockspec, extract, dest_dir)
   assert(rockspec:type() == "rockspec")
   assert(type(extract) == "boolean")
   assert(type(dest_dir) == "string" or not dest_dir)

   -- auto-convert git://github.com URLs to use git+https
   -- see https://github.blog/2021-09-01-improving-git-protocol-security-github/
   if rockspec.source.url:match("^git://github%.com/")
   or rockspec.source.url:match("^git://www%.github%.com/") then
      rockspec.source.url = rockspec.source.url:gsub("^git://", "git+https://")
      rockspec.source.protocol = "git+https"
   end

   local protocol = rockspec.source.protocol
   local ok, err, proto
   if dir.is_basic_protocol(protocol) then
      proto = fetch
   else
      ok, proto = pcall(require, "luarocks.fetch."..protocol:gsub("[+-]", "_"))
      if not ok then
         return nil, "Unknown protocol "..protocol
      end
   end

   if cfg.only_sources_from
   and rockspec.source.pathname
   and #rockspec.source.pathname > 0 then
      if #cfg.only_sources_from == 0 then
         return nil, "Can't download "..rockspec.source.url.." -- download from remote servers disabled"
      elseif rockspec.source.pathname:find(cfg.only_sources_from, 1, true) ~= 1 then
         return nil, "Can't download "..rockspec.source.url.." -- only downloading from "..cfg.only_sources_from
      end
   end

   local source_file, store_dir = proto.get_sources(rockspec, extract, dest_dir)
   if not source_file then return nil, store_dir end

   ok, err = fetch.find_rockspec_source_dir(rockspec, store_dir)
   if not ok then return nil, err, "source.dir", source_file, store_dir end

   return source_file, store_dir
end

return fetch
