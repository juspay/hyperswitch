
--- Generic utilities for handling pathnames.
local dir = {}

local core = require("luarocks.core.dir")

dir.path = core.path
dir.split_url = core.split_url
dir.normalize = core.normalize

local dir_sep = package.config:sub(1, 1)

--- Strip the path off a path+filename.
-- @param pathname string: A path+name, such as "/a/b/c"
-- or "\a\b\c".
-- @return string: The filename without its path, such as "c".
function dir.base_name(pathname)
   assert(type(pathname) == "string")

   local b
   b = pathname:gsub("[/\\]", "/") -- canonicalize to forward slashes
   b = b:gsub("/*$", "")           -- drop trailing slashes
   b = b:match(".*[/\\]([^/\\]*)") -- match last component
   b = b or pathname               -- fallback to original if no slashes

   return b
end

--- Strip the name off a path+filename.
-- @param pathname string: A path+name, such as "/a/b/c".
-- @return string: The filename without its path, such as "/a/b".
-- For entries such as "/a/b/", "/a" is returned. If there are
-- no directory separators in input, "" is returned.
function dir.dir_name(pathname)
   assert(type(pathname) == "string")

   local d
   d = pathname:gsub("[/\\]", "/") -- canonicalize to forward slashes
   d = d:gsub("/*$", "")           -- drop trailing slashes
   d = d:match("(.*)[/]+[^/]*")    -- match all components but the last
   d = d or ""                     -- switch to "" if there's no match
   d = d:gsub("/", dir_sep)        -- decanonicalize to native slashes

   return d
end

--- Returns true if protocol does not require additional tools.
-- @param protocol The protocol name
function dir.is_basic_protocol(protocol)
   return protocol == "http" or protocol == "https" or protocol == "ftp" or protocol == "file"
end

function dir.deduce_base_dir(url)
   -- for extensions like foo.tar.gz, "gz" is stripped first
   local known_exts = {}
   for _, ext in ipairs{"zip", "git", "tgz", "tar", "gz", "bz2"} do
      known_exts[ext] = ""
   end
   local base = dir.base_name(url)
   return (base:gsub("%.([^.]*)$", known_exts):gsub("%.tar", ""))
end

return dir
