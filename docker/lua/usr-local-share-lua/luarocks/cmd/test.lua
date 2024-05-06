
--- Module implementing the LuaRocks "test" command.
-- Tests a rock, compiling its C parts if any.
local cmd_test = {}

local util = require("luarocks.util")
local test = require("luarocks.test")

function cmd_test.add_to_parser(parser)
   local cmd = parser:command("test", [[
Run the test suite for the Lua project in the current directory.

If the first argument is a rockspec, it will use it to determine the parameters
for running tests; otherwise, it will attempt to detect the rockspec.

Any additional arguments are forwarded to the test suite.
To make sure that test suite flags are not interpreted as LuaRocks flags, use --
to separate LuaRocks arguments from test suite arguments.]],
      util.see_also())
      :summary("Run the test suite in the current directory.")

   cmd:argument("rockspec", "Project rockspec.")
      :args("?")
   cmd:argument("args", "Test suite arguments.")
      :args("*")
   cmd:flag("--prepare", "Only install dependencies needed for testing only, but do not run the test")

   cmd:option("--test-type", "Specify the test suite type manually if it was "..
      "not specified in the rockspec and it could not be auto-detected.")
      :argname("<type>")
end

function cmd_test.command(args)
   if args.rockspec and args.rockspec:match("rockspec$") then
      return test.run_test_suite(args.rockspec, args.test_type, args.args, args.prepare)
   end

   table.insert(args.args, 1, args.rockspec)

   local rockspec, err = util.get_default_rockspec()
   if not rockspec then
      return nil, err
   end

   return test.run_test_suite(rockspec, args.test_type, args.args, args.prepare)
end

return cmd_test
