
local dir = {}

local require = nil
--------------------------------------------------------------------------------

local dir_sep = package.config:sub(1, 1)

local function unquote(c)
   local first, last = c:sub(1,1), c:sub(-1)
   if (first == '"' and last == '"') or
      (first == "'" and last == "'") then
      return c:sub(2,-2)
   end
   return c
end

--- Describe a path in a cross-platform way.
-- Use this function to avoid platform-specific directory
-- separators in other modules. Removes trailing slashes from
-- each component given, to avoid repeated separators.
-- Separators inside strings are kept, to handle URLs containing
-- protocols.
-- @param ... strings representing directories
-- @return string: a string with a platform-specific representation
-- of the path.
function dir.path(...)
   local t = {...}
   while t[1] == "" do
      table.remove(t, 1)
   end
   for i, c in ipairs(t) do
      t[i] = unquote(c)
   end
   return dir.normalize(table.concat(t, "/"))
end

--- Split protocol and path from an URL or local pathname.
-- URLs should be in the "protocol://path" format.
-- For local pathnames, "file" is returned as the protocol.
-- @param url string: an URL or a local pathname.
-- @return string, string: the protocol, and the pathname without the protocol.
function dir.split_url(url)
   assert(type(url) == "string")

   url = unquote(url)
   local protocol, pathname = url:match("^([^:]*)://(.*)")
   if not protocol then
      protocol = "file"
      pathname = url
   end
   return protocol, pathname
end

--- Normalize a url or local path.
-- URLs should be in the "protocol://path" format.
-- Removes trailing and double slashes, and '.' and '..' components.
-- for 'file' URLs, the native system's slashes are used.
-- @param url string: an URL or a local pathname.
-- @return string: Normalized result.
function dir.normalize(name)
   local protocol, pathname = dir.split_url(name)
   pathname = pathname:gsub("\\", "/"):gsub("(.)/*$", "%1"):gsub("//", "/")
   local pieces = {}
   local drive = ""
   if pathname:match("^.:") then
      drive, pathname = pathname:match("^(.:)(.*)$")
   end
   pathname = pathname .. "/"
   for piece in pathname:gmatch("(.-)/") do
      if piece == ".." then
         local prev = pieces[#pieces]
         if not prev or prev == ".." then
            table.insert(pieces, "..")
         elseif prev ~= "" then
            table.remove(pieces)
         end
      elseif piece ~= "." then
         table.insert(pieces, piece)
      end
   end
   if #pieces == 0 then
      pathname = drive .. "."
   elseif #pieces == 1 and pieces[1] == "" then
      pathname = drive .. "/"
   else
      pathname = drive .. table.concat(pieces, "/")
   end
   if protocol ~= "file" then
      pathname = protocol .. "://" .. pathname
   else
      pathname = pathname:gsub("/", dir_sep)
   end
   return pathname
end

return dir

