local deplocks = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local util = require("luarocks.util")
local persist = require("luarocks.persist")

local deptable = {}
local deptable_mode = "start"
local deplock_abs_filename
local deplock_root_rock_name

function deplocks.init(root_rock_name, dirname)
   if deptable_mode ~= "start" then
      return
   end
   deptable_mode = "create"

   local filename = dir.path(dirname, "luarocks.lock")
   deplock_abs_filename = fs.absolute_name(filename)
   deplock_root_rock_name = root_rock_name

   deptable = {}
end

function deplocks.get_abs_filename(root_rock_name)
   if root_rock_name == deplock_root_rock_name then
      return deplock_abs_filename
   end
end

function deplocks.load(root_rock_name, dirname)
   if deptable_mode ~= "start" then
      return true, nil
   end
   deptable_mode = "locked"

   local filename = dir.path(dirname, "luarocks.lock")
   local ok, result, errcode = persist.run_file(filename, {})
   if errcode == "load" or errcode == "run" then
      -- bad config file or depends on env, so error out
      return nil, nil, "Could not read existing lockfile " .. filename
   end

   if errcode == "open" then
      -- could not open, maybe file does not exist
      return true, nil
   end

   deplock_abs_filename = fs.absolute_name(filename)
   deplock_root_rock_name = root_rock_name

   deptable = result
   return true, filename
end

function deplocks.add(depskey, name, version)
   if deptable_mode == "locked" then
      return
   end

   local dk = deptable[depskey]
   if not dk then
      dk = {}
      deptable[depskey] = dk
   end

   if not dk[name] then
      dk[name] = version
   end
end

function deplocks.get(depskey, name)
   local dk = deptable[depskey]
   if not dk then
      return nil
   end

   return deptable[name]
end

function deplocks.write_file()
   if deptable_mode ~= "create" then
      return true
   end

   return persist.save_as_module(deplock_abs_filename, deptable)
end

-- a table-like interface to deplocks
function deplocks.proxy(depskey)
   return setmetatable({}, {
      __index = function(_, k)
         return deplocks.get(depskey, k)
      end,
      __newindex = function(_, k, v)
         return deplocks.add(depskey, k, v)
      end,
   })
end

function deplocks.each(depskey)
   return util.sortedpairs(deptable[depskey] or {})
end

return deplocks
