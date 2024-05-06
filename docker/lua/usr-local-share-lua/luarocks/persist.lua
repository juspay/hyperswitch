
--- Utility module for loading files into tables and
-- saving tables into files.
local persist = {}

local core = require("luarocks.core.persist")
local util = require("luarocks.util")
local dir = require("luarocks.dir")
local fs = require("luarocks.fs")

persist.run_file = core.run_file
persist.load_into_table = core.load_into_table

local write_table

--- Write a value as Lua code.
-- This function handles only numbers and strings, invoking write_table
-- to write tables.
-- @param out table or userdata: a writer object supporting :write() method.
-- @param v: the value to be written.
-- @param level number: the indentation level
-- @param sub_order table: optional prioritization table
-- @see write_table
function persist.write_value(out, v, level, sub_order)
   if type(v) == "table" then
      level = level or 0
      write_table(out, v, level + 1, sub_order)
   elseif type(v) == "string" then
      if v:match("[\r\n]") then
         local open, close = "[[", "]]"
         local equals = 0
         local v_with_bracket = v.."]"
         while v_with_bracket:find(close, 1, true) do
            equals = equals + 1
            local eqs = ("="):rep(equals)
            open, close = "["..eqs.."[", "]"..eqs.."]"
         end
         out:write(open.."\n"..v..close)
      else
         out:write(("%q"):format(v))
      end
   else
      out:write(tostring(v))
   end
end

local is_valid_plain_key
do
   local keywords = {
      ["and"] = true,
      ["break"] = true,
      ["do"] = true,
      ["else"] = true,
      ["elseif"] = true,
      ["end"] = true,
      ["false"] = true,
      ["for"] = true,
      ["function"] = true,
      ["goto"] = true,
      ["if"] = true,
      ["in"] = true,
      ["local"] = true,
      ["nil"] = true,
      ["not"] = true,
      ["or"] = true,
      ["repeat"] = true,
      ["return"] = true,
      ["then"] = true,
      ["true"] = true,
      ["until"] = true,
      ["while"] = true,
   }
   function is_valid_plain_key(k)
      return type(k) == "string"
             and k:match("^[a-zA-Z_][a-zA-Z0-9_]*$")
             and not keywords[k]
   end
end

local function write_table_key_assignment(out, k, level)
   if is_valid_plain_key(k) then
      out:write(k)
   else
      out:write("[")
      persist.write_value(out, k, level)
      out:write("]")
   end

   out:write(" = ")
end

--- Write a table as Lua code in curly brackets notation to a writer object.
-- Only numbers, strings and tables (containing numbers, strings
-- or other recursively processed tables) are supported.
-- @param out table or userdata: a writer object supporting :write() method.
-- @param tbl table: the table to be written.
-- @param level number: the indentation level
-- @param field_order table: optional prioritization table
write_table = function(out, tbl, level, field_order)
   out:write("{")
   local sep = "\n"
   local indentation = "   "
   local indent = true
   local i = 1
   for k, v, sub_order in util.sortedpairs(tbl, field_order) do
      out:write(sep)
      if indent then
         for _ = 1, level do out:write(indentation) end
      end

      if k == i then
         i = i + 1
      else
         write_table_key_assignment(out, k, level)
      end

      persist.write_value(out, v, level, sub_order)
      if type(v) == "number" then
         sep = ", "
         indent = false
      else
         sep = ",\n"
         indent = true
      end
   end
   if sep ~= "\n" then
      out:write("\n")
      for _ = 1, level - 1 do out:write(indentation) end
   end
   out:write("}")
end

--- Write a table as series of assignments to a writer object.
-- @param out table or userdata: a writer object supporting :write() method.
-- @param tbl table: the table to be written.
-- @param field_order table: optional prioritization table
-- @return true if successful; nil and error message if failed.
local function write_table_as_assignments(out, tbl, field_order)
   for k, v, sub_order in util.sortedpairs(tbl, field_order) do
      if not is_valid_plain_key(k) then
         return nil, "cannot store '"..tostring(k).."' as a plain key."
      end
      out:write(k.." = ")
      persist.write_value(out, v, 0, sub_order)
      out:write("\n")
   end
   return true
end

--- Write a table using Lua table syntax to a writer object.
-- @param out table or userdata: a writer object supporting :write() method.
-- @param tbl table: the table to be written.
local function write_table_as_table(out, tbl)
   out:write("return {\n")
   for k, v, sub_order in util.sortedpairs(tbl) do
      out:write("   ")
      write_table_key_assignment(out, k, 1)
      persist.write_value(out, v, 1, sub_order)
      out:write(",\n")
   end
   out:write("}\n")
end

--- Save the contents of a table to a string.
-- Each element of the table is saved as a global assignment.
-- Only numbers, strings and tables (containing numbers, strings
-- or other recursively processed tables) are supported.
-- @param tbl table: the table containing the data to be written
-- @param field_order table: an optional array indicating the order of top-level fields.
-- @return persisted data as string; or nil and an error message
function persist.save_from_table_to_string(tbl, field_order)
   local out = {buffer = {}}
   function out:write(data) table.insert(self.buffer, data) end
   local ok, err = write_table_as_assignments(out, tbl, field_order)
   if not ok then
      return nil, err
   end
   return table.concat(out.buffer)
end

--- Save the contents of a table in a file.
-- Each element of the table is saved as a global assignment.
-- Only numbers, strings and tables (containing numbers, strings
-- or other recursively processed tables) are supported.
-- @param filename string: the output filename
-- @param tbl table: the table containing the data to be written
-- @param field_order table: an optional array indicating the order of top-level fields.
-- @return boolean or (nil, string): true if successful, or nil and a
-- message in case of errors.
function persist.save_from_table(filename, tbl, field_order)
   local prefix = dir.dir_name(filename)
   fs.make_dir(prefix)
   local out = io.open(filename, "w")
   if not out then
      return nil, "Cannot create file at "..filename
   end
   local ok, err = write_table_as_assignments(out, tbl, field_order)
   out:close()
   if not ok then
      return nil, err
   end
   return true
end

--- Save the contents of a table as a module.
-- The module contains a 'return' statement that returns the table.
-- Only numbers, strings and tables (containing numbers, strings
-- or other recursively processed tables) are supported.
-- @param filename string: the output filename
-- @param tbl table: the table containing the data to be written
-- @return boolean or (nil, string): true if successful, or nil and a
-- message in case of errors.
function persist.save_as_module(filename, tbl)
   local out = io.open(filename, "w")
   if not out then
      return nil, "Cannot create file at "..filename
   end
   write_table_as_table(out, tbl)
   out:close()
   return true
end

function persist.load_config_file_if_basic(filename, cfg)
   local env = {
      home = cfg.home
   }
   local result, err, errcode = persist.load_into_table(filename, env)
   if errcode == "load" or errcode == "run" then
      -- bad config file or depends on env, so error out
      return nil, "Could not read existing config file " .. filename
   end

   local tbl
   if errcode == "open" then
      -- could not open, maybe file does not exist
      tbl = {}
   else
      tbl = result
      tbl.home = nil
   end

   return tbl
end

function persist.save_default_lua_version(prefix, lua_version)
   local ok, err = fs.make_dir(prefix)
   if not ok then
      return nil, err
   end
   local fd, err = io.open(dir.path(prefix, "default-lua-version.lua"), "w")
   if not fd then
      return nil, err
   end
   fd:write('return "' .. lua_version .. '"\n')
   fd:close()
   return true
end

return persist
