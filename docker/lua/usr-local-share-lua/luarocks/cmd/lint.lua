
--- Module implementing the LuaRocks "lint" command.
-- Utility function that checks syntax of the rockspec.
local lint = {}

local util = require("luarocks.util")
local download = require("luarocks.download")
local fetch = require("luarocks.fetch")

function lint.add_to_parser(parser)
   local cmd = parser:command("lint", "Check syntax of a rockspec.\n\n"..
      "Returns success if the text of the rockspec is syntactically correct, else failure.",
      util.see_also())
      :summary("Check syntax of a rockspec.")

   cmd:argument("rockspec", "The rockspec to check.")
end

function lint.command(args)

   local filename = args.rockspec
   if not filename:match(".rockspec$") then
      local err
      filename, err = download.download("rockspec", filename:lower())
      if not filename then
         return nil, err
      end
   end

   local rs, err = fetch.load_local_rockspec(filename)
   if not rs then
      return nil, "Failed loading rockspec: "..err
   end

   local ok = true

   -- This should have been done in the type checker,
   -- but it would break compatibility of other commands.
   -- Making 'lint' alone be stricter shouldn't be a problem,
   -- because extra-strict checks is what lint-type commands
   -- are all about.
   if not rs.description or not rs.description.license then
      util.printerr("Rockspec has no description.license field.")
      ok = false
   end

   return ok, ok or filename.." failed consistency checks."
end

return lint
