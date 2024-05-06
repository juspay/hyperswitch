local signing = {}

local cfg = require("luarocks.core.cfg")
local fs = require("luarocks.fs")

local function get_gpg()
   local vars = cfg.variables
   local gpg = vars.GPG
   local gpg_ok, err = fs.is_tool_available(gpg, "gpg")
   if not gpg_ok then
      return nil, err
   end
   return gpg
end

function signing.signature_url(url)
   return url .. ".asc"
end

function signing.sign_file(file)
   local gpg, err = get_gpg()
   if not gpg then
      return nil, err
   end

   local sigfile = file .. ".asc"
   if fs.execute(gpg, "--armor", "--output", sigfile, "--detach-sign", file) then
      return sigfile
   else
      return nil, "failed running " .. gpg .. " to sign " .. file
   end
end

function signing.verify_signature(file, sigfile)
   local gpg, err = get_gpg()
   if not gpg then
      return nil, err
   end

   if fs.execute(gpg, "--verify", sigfile, file) then
      return true
   else
      return nil, "GPG returned a verification error"
   end

end

return signing
