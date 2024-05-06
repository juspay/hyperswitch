
--- Proxy module for filesystem and platform abstractions.
-- All code using "fs" code should require "luarocks.fs",
-- and not the various platform-specific implementations.
-- However, see the documentation of the implementation
-- for the API reference.

local pairs = pairs

local fs = {}
-- To avoid a loop when loading the other fs modules.
package.loaded["luarocks.fs"] = fs

local cfg = require("luarocks.core.cfg")

local pack = table.pack or function(...) return { n = select("#", ...), ... } end

math.randomseed(os.time())

local fs_is_verbose = false

do
   local old_popen, old_execute

   -- patch io.popen and os.execute to display commands in verbose mode
   function fs.verbose()
      fs_is_verbose = true

      if old_popen or old_execute then return end
      old_popen = io.popen
      -- luacheck: push globals io os
      io.popen = function(one, two)
         if two == nil then
            print("\nio.popen: ", one)
         else
            print("\nio.popen: ", one, "Mode:", two)
         end
         return old_popen(one, two)
      end

      old_execute = os.execute
      os.execute = function(cmd)
         -- redact api keys if present
         print("\nos.execute: ", (cmd:gsub("(/api/[^/]+/)([^/]+)/", function(cap, key) return cap.."<redacted>/" end)) )
         local a, b, c = old_execute(cmd)
         if type(a) == "boolean" then
            print((a and ".........." or "##########") .. ": " .. tostring(c) .. (b == "exit" and "" or " (" .. tostring(b) .. ")"))
         elseif type(a) == "number" then
            print(((a == 0) and ".........." or "##########") .. ": " .. tostring(a))
         end
         return a, b, c
      end
      -- luacheck: pop
   end
end

do
   local skip_verbose_wrap = {
      ["current_dir"] = true,
   }

   local function load_fns(fs_table, inits)
      for name, fn in pairs(fs_table) do
         if name ~= "init" and not fs[name] then
            if skip_verbose_wrap[name] then
               fs[name] = fn
            else
               fs[name] = function(...)
                  if fs_is_verbose then
                     local args = pack(...)
                     for i=1, args.n do
                        local arg = args[i]
                        local pok, v = pcall(string.format, "%q", arg)
                        args[i] = pok and v or tostring(arg)
                     end
                     print("fs." .. name .. "(" .. table.concat(args, ", ") .. ")")
                  end
                  return fn(...)
               end
            end
         end
      end
      if fs_table.init then
         table.insert(inits, fs_table.init)
      end
   end

   local function load_platform_fns(patt, inits)
      local each_platform = cfg.each_platform

      -- FIXME A quick hack for the experimental Windows build
      if os.getenv("LUAROCKS_CROSS_COMPILING") then
         each_platform = function()
            local i = 0
            local plats = { "linux", "unix" }
            return function()
               i = i + 1
               return plats[i]
            end
         end
      end

      for platform in each_platform("most-specific-first") do
         local ok, fs_plat = pcall(require, patt:format(platform))
         if ok and fs_plat then
            load_fns(fs_plat, inits)
         end
      end
   end

   function fs.init()
      local inits = {}

      if fs.current_dir then
         -- unload luarocks fs so it can be reloaded using all modules
         -- providing extra functionality in the current package paths
         for k, _ in pairs(fs) do
            if k ~= "init" and k ~= "verbose" then
               fs[k] = nil
            end
         end
         for m, _ in pairs(package.loaded) do
            if m:match("luarocks%.fs%.") then
               package.loaded[m] = nil
            end
         end
      end

      -- Load platform-specific functions
      load_platform_fns("luarocks.fs.%s", inits)

      -- Load platform-independent pure-Lua functionality
      load_fns(require("luarocks.fs.lua"), inits)

      -- Load platform-specific fallbacks for missing Lua modules
      load_platform_fns("luarocks.fs.%s.tools", inits)

      -- Load platform-independent external tool functionality
      load_fns(require("luarocks.fs.tools"), inits)

      -- Run platform-specific initializations after everything is loaded
      for _, init in ipairs(inits) do
         init()
      end
   end
end

return fs
