
local test = {}

local fetch = require("luarocks.fetch")
local deps = require("luarocks.deps")
local util = require("luarocks.util")

local test_types = {
   "busted",
   "command",
}

local test_modules = {}

for _, test_type in ipairs(test_types) do
   local mod = require("luarocks.test." .. test_type)
   table.insert(test_modules, mod)
   test_modules[test_type] = mod
   test_modules[mod] = test_type
end

local function get_test_type(rockspec)
   if rockspec.test and rockspec.test.type then
      return rockspec.test.type
   end

   for _, test_module in ipairs(test_modules) do
      if test_module.detect_type() then
         return test_modules[test_module]
      end
   end

   return nil, "could not detect test type -- no test suite for " .. rockspec.package .. "?"
end

-- Run test suite as configured in rockspec in the current directory.
function test.run_test_suite(rockspec_arg, test_type, args, prepare)
   local rockspec
   if type(rockspec_arg) == "string" then
      local err, errcode
      rockspec, err, errcode = fetch.load_rockspec(rockspec_arg)
      if err then
         return nil, err, errcode
      end
   else
      assert(type(rockspec_arg) == "table")
      rockspec = rockspec_arg
   end

   if not test_type then
      local err
      test_type, err = get_test_type(rockspec, test_type)
      if not test_type then
         return nil, err
      end
   end
   assert(test_type)

   local all_deps = {
      "dependencies",
      "build_dependencies",
      "test_dependencies",
   }
   for _, dep_kind in ipairs(all_deps) do
      if rockspec[dep_kind] and next(rockspec[dep_kind]) then
         local ok, err, errcode = deps.fulfill_dependencies(rockspec, dep_kind, "all")
         if err then
            return nil, err, errcode
         end
      end
   end

   local mod_name = "luarocks.test." .. test_type
   local pok, test_mod = pcall(require, mod_name)
   if not pok then
      return nil, "failed loading test execution module " .. mod_name
   end

   if prepare then
      if test_type == "busted" then
         return test_mod.run_tests(rockspec_arg, {"--version"})
      else
         return true
      end
   else
      local flags = rockspec.test and rockspec.test.flags
      if type(flags) == "table" then
         util.variable_substitutions(flags, rockspec.variables)

         -- insert any flags given in test.flags at the front of args
         for i = 1, #flags do
            table.insert(args, i, flags[i])
         end
      end

      return test_mod.run_tests(rockspec.test, args)
   end
end

return test
