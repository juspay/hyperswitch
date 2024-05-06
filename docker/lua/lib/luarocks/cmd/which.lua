
--- @module luarocks.which_cmd
-- Driver for the `luarocks which` command.
local which_cmd = {}

local loader = require("luarocks.loader")
local cfg = require("luarocks.core.cfg")
local util = require("luarocks.util")

function which_cmd.add_to_parser(parser)
   local cmd = parser:command("which", 'Given a module name like "foo.bar", '..
      "output which file would be loaded to resolve that module by "..
      'luarocks.loader, like "/usr/local/lua/'..cfg.lua_version..'/foo/bar.lua".',
      util.see_also())
      :summary("Tell which file corresponds to a given module name.")

   cmd:argument("modname", "Module name.")
end

--- Driver function for "which" command.
-- @return boolean This function terminates the interpreter.
function which_cmd.command(args)
   local pathname, rock_name, rock_version, where = loader.which(args.modname, "lp")

   if pathname then
      util.printout(pathname)
      if where == "l" then
         util.printout("(provided by " .. tostring(rock_name) .. " " .. tostring(rock_version) .. ")")
      else
         local key = rock_name
         util.printout("(found directly via package." .. key.. " -- not installed as a rock?)")
      end
      return true
   end

   return nil, "Module '" .. args.modname .. "' not found."
end

return which_cmd

