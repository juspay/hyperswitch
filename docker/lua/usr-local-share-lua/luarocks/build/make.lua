
--- Build back-end for using Makefile-based packages.
local make = {}

local unpack = unpack or table.unpack

local fs = require("luarocks.fs")
local util = require("luarocks.util")
local cfg = require("luarocks.core.cfg")

--- Call "make" with given target and variables
-- @param make_cmd string: the make command to be used (typically
-- configured through variables.MAKE in the config files, or
-- the appropriate platform-specific default).
-- @param pass boolean: If true, run make; if false, do nothing.
-- @param target string: The make target; an empty string indicates
-- the default target.
-- @param variables table: A table containing string-string key-value
-- pairs representing variable assignments to be passed to make.
-- @return boolean: false if any errors occurred, true otherwise.
local function make_pass(make_cmd, pass, target, variables)
   assert(type(pass) == "boolean")
   assert(type(target) == "string")
   assert(type(variables) == "table")

   local assignments = {}
   for k,v in pairs(variables) do
      table.insert(assignments, k.."="..v)
   end
   if pass then
      return fs.execute(make_cmd.." "..target, unpack(assignments))
   else
      return true
   end
end

--- Driver function for the "make" build back-end.
-- @param rockspec table: the loaded rockspec.
-- @return boolean or (nil, string): true if no errors occurred,
-- nil and an error message otherwise.
function make.run(rockspec, not_install)
   assert(rockspec:type() == "rockspec")

   local build = rockspec.build

   if build.build_pass == nil then build.build_pass = true end
   if build.install_pass == nil then build.install_pass = true end
   build.build_variables = build.build_variables or {}
   build.install_variables = build.install_variables or {}
   build.build_target = build.build_target or ""
   build.install_target = build.install_target or "install"
   local makefile = build.makefile or cfg.makefile
   if makefile then
      -- Assumes all make's accept -f. True for POSIX make, GNU make and Microsoft nmake.
      build.build_target = "-f "..makefile.." "..build.build_target
      build.install_target = "-f "..makefile.." "..build.install_target
   end

   if build.variables then
      for var, val in pairs(build.variables) do
         build.build_variables[var] = val
         build.install_variables[var] = val
      end
   end

   util.warn_if_not_used(build.build_variables, { CFLAGS=true }, "variable %s was not passed in build_variables")

   util.variable_substitutions(build.build_variables, rockspec.variables)
   util.variable_substitutions(build.install_variables, rockspec.variables)

   local auto_variables = { "CC" }

   for _, variable in pairs(auto_variables) do
      if not build.build_variables[variable] then
         build.build_variables[variable] = rockspec.variables[variable]
      end
      if not build.install_variables[variable] then
         build.install_variables[variable] = rockspec.variables[variable]
      end
   end

   -- backwards compatibility
   local make_cmd = cfg.make or rockspec.variables.MAKE

   local ok = make_pass(make_cmd, build.build_pass, build.build_target, build.build_variables)
   if not ok then
      return nil, "Failed building."
   end
   if not not_install then
      ok = make_pass(make_cmd, build.install_pass, build.install_target, build.install_variables)
      if not ok then
         return nil, "Failed installing."
      end
   end
   return true
end

return make
