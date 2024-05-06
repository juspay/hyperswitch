
local util = {}

local require = nil
--------------------------------------------------------------------------------

local dir_sep = package.config:sub(1, 1)

--- Run a process and read a its output.
-- Equivalent to io.popen(cmd):read("*l"), except that it
-- closes the fd right away.
-- @param cmd string: The command to execute
-- @param spec string: "*l" by default, to read a single line.
-- May be used to read more, passing, for instance, "*a".
-- @return string: the output of the program.
function util.popen_read(cmd, spec)
   local tmpfile = (dir_sep == "\\")
                   and (os.getenv("TMP") .. "/luarocks-" .. tostring(math.floor(math.random() * 10000)))
                   or os.tmpname()
   os.execute(cmd .. " > " .. tmpfile)
   local fd = io.open(tmpfile, "rb")
   if not fd then
      os.remove(tmpfile)
      return ""
   end
   local out = fd:read(spec or "*l")
   fd:close()
   os.remove(tmpfile)
   return out or ""
end

---
-- Formats tables with cycles recursively to any depth.
-- References to other tables are shown as values.
-- Self references are indicated.
-- The string returned is "Lua code", which can be processed
-- (in the case in which indent is composed by spaces or "--").
-- Userdata and function keys and values are shown as strings,
-- which logically are exactly not equivalent to the original code.
-- This routine can serve for pretty formating tables with
-- proper indentations, apart from printing them:
-- io.write(table.show(t, "t"))   -- a typical use
-- Written by Julio Manuel Fernandez-Diaz,
-- Heavily based on "Saving tables with cycles", PIL2, p. 113.
-- @param t table: is the table.
-- @param tname string: is the name of the table (optional)
-- @param top_indent string: is a first indentation (optional).
-- @return string: the pretty-printed table
function util.show_table(t, tname, top_indent)
   local cart     -- a container
   local autoref  -- for self references

   local function is_empty_table(tbl) return next(tbl) == nil end

   local function basic_serialize(o)
      local so = tostring(o)
      if type(o) == "function" then
         local info = debug and debug.getinfo(o, "S")
         if not info then
            return ("%q"):format(so)
         end
         -- info.name is nil because o is not a calling level
         if info.what == "C" then
            return ("%q"):format(so .. ", C function")
         else
            -- the information is defined through lines
            return ("%q"):format(so .. ", defined in (" .. info.linedefined .. "-" .. info.lastlinedefined .. ")" .. info.source)
         end
      elseif type(o) == "number" then
         return so
      else
         return ("%q"):format(so)
      end
   end

   local function add_to_cart(value, name, indent, saved, field)
      indent = indent or ""
      saved = saved or {}
      field = field or name

      cart = cart .. indent .. field

      if type(value) ~= "table" then
         cart = cart .. " = " .. basic_serialize(value) .. ";\n"
      else
         if saved[value] then
            cart = cart .. " = {}; -- " .. saved[value] .. " (self reference)\n"
            autoref = autoref ..  name .. " = " .. saved[value] .. ";\n"
         else
            saved[value] = name
            if is_empty_table(value) then
               cart = cart .. " = {};\n"
            else
               cart = cart .. " = {\n"
               for k, v in pairs(value) do
                  k = basic_serialize(k)
                  local fname = ("%s[%s]"):format(name, k)
                  field = ("[%s]"):format(k)
                  -- three spaces between levels
                  add_to_cart(v, fname, indent .. "   ", saved, field)
               end
               cart = cart .. indent .. "};\n"
            end
         end
      end
   end

   tname = tname or "__unnamed__"
   if type(t) ~= "table" then
      return tname .. " = " .. basic_serialize(t)
   end
   cart, autoref = "", ""
   add_to_cart(t, tname, top_indent)
   return cart .. autoref
end

--- Merges contents of src on top of dst's contents
-- (i.e. if an key from src already exists in dst, replace it).
-- @param dst Destination table, which will receive src's contents.
-- @param src Table which provides new contents to dst.
function util.deep_merge(dst, src)
   for k, v in pairs(src) do
      if type(v) == "table" then
         if dst[k] == nil then
            dst[k] = {}
         end
         if type(dst[k]) == "table" then
            util.deep_merge(dst[k], v)
         else
            dst[k] = v
         end
      else
         dst[k] = v
      end
   end
end

--- Merges contents of src below those of dst's contents
-- (i.e. if an key from src already exists in dst, do not replace it).
-- @param dst Destination table, which will receive src's contents.
-- @param src Table which provides new contents to dst.
function util.deep_merge_under(dst, src)
   for k, v in pairs(src) do
      if type(v) == "table" then
         if dst[k] == nil then
            dst[k] = {}
         end
         if type(dst[k]) == "table" then
            util.deep_merge_under(dst[k], v)
         end
      elseif dst[k] == nil then
         dst[k] = v
      end
   end
end

--- Clean up a path-style string ($PATH, $LUA_PATH/package.path, etc.),
-- removing repeated entries and making sure only the relevant
-- Lua version is used.
-- Example: given ("a;b;c;a;b;d", ";"), returns "a;b;c;d".
-- @param list string: A path string (from $PATH or package.path)
-- @param sep string: The separator
-- @param lua_version (optional) string: The Lua version to use.
-- @param keep_first (optional) if true, keep first occurrence in case
-- of duplicates; otherwise keep last occurrence. The default is false.
function util.cleanup_path(list, sep, lua_version, keep_first)
   assert(type(list) == "string")
   assert(type(sep) == "string")

   list = list:gsub(dir_sep, "/")

   local parts = util.split_string(list, sep)
   local final, entries = {}, {}
   local start, stop, step

   if keep_first then
      start, stop, step = 1, #parts, 1
   else
      start, stop, step = #parts, 1, -1
   end

   for i = start, stop, step do
      local part = parts[i]:gsub("//", "/")
      if lua_version then
         part = part:gsub("/lua/([%d.]+)/", function(part_version)
            if part_version:sub(1, #lua_version) ~= lua_version then
               return "/lua/"..lua_version.."/"
            end
         end)
      end
      if not entries[part] then
         local at = keep_first and #final+1 or 1
         table.insert(final, at, part)
         entries[part] = true
      end
   end

   return (table.concat(final, sep):gsub("/", dir_sep))
end

-- from http://lua-users.org/wiki/SplitJoin
-- by Philippe Lhoste
function util.split_string(str, delim, maxNb)
   -- Eliminate bad cases...
   if string.find(str, delim) == nil then
      return { str }
   end
   if maxNb == nil or maxNb < 1 then
      maxNb = 0    -- No limit
   end
   local result = {}
   local pat = "(.-)" .. delim .. "()"
   local nb = 0
   local lastPos
   for part, pos in string.gmatch(str, pat) do
      nb = nb + 1
      result[nb] = part
      lastPos = pos
      if nb == maxNb then break end
   end
   -- Handle the last field
   if nb ~= maxNb then
      result[nb + 1] = string.sub(str, lastPos)
   end
   return result
end

--- Return an array of keys of a table.
-- @param tbl table: The input table.
-- @return table: The array of keys.
function util.keys(tbl)
   local ks = {}
   for k,_ in pairs(tbl) do
      table.insert(ks, k)
   end
   return ks
end

--- Print a line to standard error
function util.printerr(...)
   io.stderr:write(table.concat({...},"\t"))
   io.stderr:write("\n")
end

--- Display a warning message.
-- @param msg string: the warning message
function util.warning(msg)
   util.printerr("Warning: "..msg)
end

--- Simple sort function used as a default for util.sortedpairs.
local function default_sort(a, b)
   local ta = type(a)
   local tb = type(b)
   if ta == "number" and tb == "number" then
      return a < b
   elseif ta == "number" then
      return true
   elseif tb == "number" then
      return false
   else
      return tostring(a) < tostring(b)
   end
end

--- A table iterator generator that returns elements sorted by key,
-- to be used in "for" loops.
-- @param tbl table: The table to be iterated.
-- @param sort_function function or table or nil: An optional comparison function
-- to be used by table.sort when sorting keys, or an array listing an explicit order
-- for keys. If a value itself is an array, it is taken so that the first element
-- is a string representing the field name, and the second element is a priority table
-- for that key, which is returned by the iterator as the third value after the key
-- and the value.
-- @return function: the iterator function.
function util.sortedpairs(tbl, sort_function)
   sort_function = sort_function or default_sort
   local keys = util.keys(tbl)
   local sub_orders = {}

   if type(sort_function) == "function" then
      table.sort(keys, sort_function)
   else
      local order = sort_function
      local ordered_keys = {}
      local all_keys = keys
      keys = {}

      for _, order_entry in ipairs(order) do
         local key, sub_order
         if type(order_entry) == "table" then
            key = order_entry[1]
            sub_order = order_entry[2]
         else
            key = order_entry
         end

         if tbl[key] then
            ordered_keys[key] = true
            sub_orders[key] = sub_order
            table.insert(keys, key)
         end
      end

      table.sort(all_keys, default_sort)
      for _, key in ipairs(all_keys) do
         if not ordered_keys[key] then
            table.insert(keys, key)
         end
      end
   end

   local i = 1
   return function()
      local key = keys[i]
      i = i + 1
      return key, tbl[key], sub_orders[key]
   end
end

return util

