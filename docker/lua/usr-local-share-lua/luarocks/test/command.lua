
local command = {}

local fs = require("luarocks.fs")
local cfg = require("luarocks.core.cfg")

local unpack = table.unpack or unpack

function command.detect_type()
   if fs.exists("test.lua") then
      return true
   end
   return false
end

function command.run_tests(test, args)
   if not test then
      test = {
         script = "test.lua"
      }
   end

   if not test.script and not test.command then
      test.script = "test.lua"
   end

   local ok

   if test.script then
      if type(test.script) ~= "string" then
         return nil, "Malformed rockspec: 'script' expects a string"
      end
      if not fs.exists(test.script) then
         return nil, "Test script " .. test.script .. " does not exist"
      end
      local lua = fs.Q(cfg.variables["LUA"])  -- get lua interpreter configured
      ok = fs.execute(lua, test.script, unpack(args))
   elseif test.command then
      if type(test.command) ~= "string" then
         return nil, "Malformed rockspec: 'command' expects a string"
      end
      ok = fs.execute(test.command, unpack(args))
   end

   if ok then
      return true
   else
      return nil, "tests failed with non-zero exit code"
   end
end

return command
