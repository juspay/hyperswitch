
local persist = {}

local require = nil
--------------------------------------------------------------------------------

--- Load and run a Lua file in an environment.
-- @param filename string: the name of the file.
-- @param env table: the environment table.
-- @return (true, any) or (nil, string, string): true and the return value
-- of the file, or nil, an error message and an error code ("open", "load"
-- or "run") in case of errors.
function persist.run_file(filename, env)
   local fd, err = io.open(filename)
   if not fd then
      return nil, err, "open"
   end
   local str, err = fd:read("*a")
   fd:close()
   if not str then
      return nil, err, "open"
   end
   str = str:gsub("^#![^\n]*\n", "")
   local chunk, ran
   if _VERSION == "Lua 5.1" then -- Lua 5.1
      chunk, err = loadstring(str, filename)
      if chunk then
         setfenv(chunk, env)
         ran, err = pcall(chunk)
      end
   else -- Lua 5.2
      chunk, err = load(str, filename, "t", env)
      if chunk then
         ran, err = pcall(chunk)
      end
   end
   if not chunk then
      return nil, "Error loading file: "..err, "load"
   end
   if not ran then
      return nil, "Error running file: "..err, "run"
   end
   return true, err
end

--- Load a Lua file containing assignments, storing them in a table.
-- The global environment is not propagated to the loaded file.
-- @param filename string: the name of the file.
-- @param tbl table or nil: if given, this table is used to store
-- loaded values.
-- @return (table, table) or (nil, string, string): a table with the file's assignments
-- as fields and set of undefined globals accessed in file,
-- or nil, an error message and an error code ("open"; couldn't open the file,
-- "load"; compile-time error, or "run"; run-time error)
-- in case of errors.
function persist.load_into_table(filename, tbl)
   assert(type(filename) == "string")
   assert(type(tbl) == "table" or not tbl)

   local result = tbl or {}
   local globals = {}
   local globals_mt = {
      __index = function(t, k)
         globals[k] = true
      end
   }
   local save_mt = getmetatable(result)
   setmetatable(result, globals_mt)

   local ok, err, errcode = persist.run_file(filename, result)

   setmetatable(result, save_mt)

   if not ok then
      return nil, err, errcode
   end
   return result, globals
end

return persist

