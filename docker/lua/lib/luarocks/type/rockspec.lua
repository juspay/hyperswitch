local type_rockspec = {}

local type_check = require("luarocks.type_check")

type_rockspec.rockspec_format = "3.0"

-- Syntax for type-checking tables:
--
-- A type-checking table describes typing data for a value.
-- Any key starting with an underscore has a special meaning:
-- _type (string) is the Lua type of the value. Default is "table".
-- _mandatory (boolean) indicates if the value is a mandatory key in its container table. Default is false.
-- For "string" types only:
--    _pattern (string) is the string-matching pattern, valid for string types only. Default is ".*".
-- For "table" types only:
--    _any (table) is the type-checking table for unspecified keys, recursively checked.
--    _more (boolean) indicates that the table accepts unspecified keys and does not type-check them.
--    Any other string keys that don't start with an underscore represent known keys and are type-checking tables, recursively checked.

local rockspec_formats, versions = type_check.declare_schemas({
   ["1.0"] = {
      rockspec_format = { _type = "string" },
      package = { _type = "string", _mandatory = true },
      version = { _type = "string", _pattern = "[%w.]+-[%d]+", _mandatory = true },
      description = {
         summary = { _type = "string" },
         detailed = { _type = "string" },
         homepage = { _type = "string" },
         license = { _type = "string" },
         maintainer = { _type = "string" },
      },
      dependencies = {
         platforms = type_check.MAGIC_PLATFORMS,
         _any = {
            _type = "string",
            _name = "a valid dependency string",
            _pattern = "%s*([a-zA-Z0-9][a-zA-Z0-9%.%-%_]*)%s*([^/]*)",
         },
      },
      supported_platforms = {
         _any = { _type = "string" },
      },
      external_dependencies = {
         platforms = type_check.MAGIC_PLATFORMS,
         _any = {
            program = { _type = "string" },
            header = { _type = "string" },
            library = { _type = "string" },
         }
      },
      source = {
         _mandatory = true,
         platforms = type_check.MAGIC_PLATFORMS,
         url = { _type = "string", _mandatory = true },
         md5 = { _type = "string" },
         file = { _type = "string" },
         dir = { _type = "string" },
         tag = { _type = "string" },
         branch = { _type = "string" },
         module = { _type = "string" },
         cvs_tag = { _type = "string" },
         cvs_module = { _type = "string" },
      },
      build = {
         platforms = type_check.MAGIC_PLATFORMS,
         type = { _type = "string" },
         install = {
            lua = {
               _more = true
            },
            lib = {
               _more = true
            },
            conf = {
               _more = true
            },
            bin = {
               _more = true
            }
         },
         copy_directories = {
            _any = { _type = "string" },
         },
         _more = true,
         _mandatory = true
      },
      hooks = {
         platforms = type_check.MAGIC_PLATFORMS,
         post_install = { _type = "string" },
      },
   },

   ["1.1"] = {
      deploy = {
         wrap_bin_scripts = { _type = "boolean" },
      }
   },

   ["3.0"] = {
      description = {
         labels = {
            _any = { _type = "string" }
         },
         issues_url = { _type = "string" },
      },
      dependencies = {
         _any = {
            _pattern = "%s*([a-zA-Z0-9%.%-%_]*/?[a-zA-Z0-9][a-zA-Z0-9%.%-%_]*)%s*([^/]*)",
         },
      },
      build_dependencies = {
         platforms = type_check.MAGIC_PLATFORMS,
         _any = {
            _type = "string",
            _name = "a valid dependency string",
            _pattern = "%s*([a-zA-Z0-9%.%-%_]*/?[a-zA-Z0-9][a-zA-Z0-9%.%-%_]*)%s*([^/]*)",
         },
      },
      test_dependencies = {
         platforms = type_check.MAGIC_PLATFORMS,
         _any = {
            _type = "string",
            _name = "a valid dependency string",
            _pattern = "%s*([a-zA-Z0-9%.%-%_]*/?[a-zA-Z0-9][a-zA-Z0-9%.%-%_]*)%s*([^/]*)",
         },
      },
      build = {
         _mandatory = false,
      },
      test = {
         platforms = type_check.MAGIC_PLATFORMS,
         type = { _type = "string" },
         _more = true,
      },
   }
})

type_rockspec.order = {"rockspec_format", "package", "version",
   { "source", { "url", "tag", "branch", "md5" } },
   { "description", {"summary", "detailed", "homepage", "license" } },
   "supported_platforms", "dependencies", "build_dependencies", "external_dependencies",
   { "build", {"type", "modules", "copy_directories", "platforms"} },
   "test_dependencies", { "test", {"type"} },
   "hooks"}

local function check_rockspec_using_version(rockspec, globals, version)
   local schema = rockspec_formats[version]
   if not schema then
      return nil, "unknown rockspec format " .. version
   end
   local ok, err = type_check.check_undeclared_globals(globals, schema)
   if ok then
      ok, err = type_check.type_check_table(version, rockspec, schema, "")
   end
   if ok then
      return true
   else
      return nil, err
   end
end

--- Type check a rockspec table.
-- Verify the correctness of elements from a
-- rockspec table, reporting on unknown fields and type
-- mismatches.
-- @return boolean or (nil, string): true if type checking
-- succeeded, or nil and an error message if it failed.
function type_rockspec.check(rockspec, globals)
   assert(type(rockspec) == "table")

   local version = rockspec.rockspec_format or "1.0"
   local ok, err = check_rockspec_using_version(rockspec, globals, version)
   if ok then
      return true
   end

   -- Rockspec parsing failed.
   -- Let's see if it would pass using a later version.

   local found = false
   for _, v in ipairs(versions) do
      if not found then
         if v == version then
            found = true
         end
      else
         local v_ok, v_err = check_rockspec_using_version(rockspec, globals, v)
         if v_ok then
            return nil, err .. " (using rockspec format " .. version .. " -- " ..
               [[adding 'rockspec_format = "]] .. v .. [["' to the rockspec ]] ..
               [[will fix this)]]
         end
      end
   end

   return nil, err .. " (using rockspec format " .. version .. ")"
end

return type_rockspec
