
--- A pure-Lua implementation of untar (unpacking .tar archives)
local tar = {}

local fs = require("luarocks.fs")
local dir = require("luarocks.dir")
local fun = require("luarocks.fun")

local blocksize = 512

local function get_typeflag(flag)
   if flag == "0" or flag == "\0" then return "file"
   elseif flag == "1" then return "link"
   elseif flag == "2" then return "symlink" -- "reserved" in POSIX, "symlink" in GNU
   elseif flag == "3" then return "character"
   elseif flag == "4" then return "block"
   elseif flag == "5" then return "directory"
   elseif flag == "6" then return "fifo"
   elseif flag == "7" then return "contiguous" -- "reserved" in POSIX, "contiguous" in GNU
   elseif flag == "x" then return "next file"
   elseif flag == "g" then return "global extended header"
   elseif flag == "L" then return "long name"
   elseif flag == "K" then return "long link name"
   end
   return "unknown"
end

local function octal_to_number(octal)
   local exp = 0
   local number = 0
   octal = octal:gsub("%s", "")
   for i = #octal,1,-1 do
      local digit = tonumber(octal:sub(i,i))
      if not digit then
         break
      end
      number = number + (digit * 8^exp)
      exp = exp + 1
   end
   return number
end

local function checksum_header(block)
   local sum = 256

   if block:byte(1) == 0 then
      return 0
   end

   for i = 1,148 do
      local b = block:byte(i) or 0
      sum = sum + b
   end
   for i = 157,500 do
      local b = block:byte(i) or 0
      sum = sum + b
   end

   return sum
end

local function nullterm(s)
   return s:match("^[^%z]*")
end

local function read_header_block(block)
   local header = {}
   header.name = nullterm(block:sub(1,100))
   header.mode = nullterm(block:sub(101,108)):gsub(" ", "")
   header.uid = octal_to_number(nullterm(block:sub(109,116)))
   header.gid = octal_to_number(nullterm(block:sub(117,124)))
   header.size = octal_to_number(nullterm(block:sub(125,136)))
   header.mtime = octal_to_number(nullterm(block:sub(137,148)))
   header.chksum = octal_to_number(nullterm(block:sub(149,156)))
   header.typeflag = get_typeflag(block:sub(157,157))
   header.linkname = nullterm(block:sub(158,257))
   header.magic = block:sub(258,263)
   header.version = block:sub(264,265)
   header.uname = nullterm(block:sub(266,297))
   header.gname = nullterm(block:sub(298,329))
   header.devmajor = octal_to_number(nullterm(block:sub(330,337)))
   header.devminor = octal_to_number(nullterm(block:sub(338,345)))
   header.prefix = block:sub(346,500)

   -- if header.magic ~= "ustar " and header.magic ~= "ustar\0" then
   --    return false, ("Invalid header magic %6x"):format(bestring_to_number(header.magic))
   -- end
   -- if header.version ~= "00" and header.version ~= " \0" then
   --    return false, "Unknown version "..header.version
   -- end
   if header.typeflag == "unknown" then
      if checksum_header(block) ~= header.chksum then
         return false, "Failed header checksum"
      end
   end
   return header
end

function tar.untar(filename, destdir)
   assert(type(filename) == "string")
   assert(type(destdir) == "string")

   local tar_handle = io.open(filename, "rb")
   if not tar_handle then return nil, "Error opening file "..filename end

   local long_name, long_link_name
   local ok, err
   local make_dir = fun.memoize(fs.make_dir)
   while true do
      local block
      repeat
         block = tar_handle:read(blocksize)
      until (not block) or block:byte(1) > 0
      if not block then break end
      if #block < blocksize then
         ok, err = nil, "Invalid block size -- corrupted file?"
         break
      end

      local header
      header, err = read_header_block(block)
      if not header then
         ok = false
         break
      end

      local file_data = ""
      if header.size > 0 then
         local nread = math.ceil(header.size / blocksize) * blocksize
         file_data = tar_handle:read(header.size)
         if nread > header.size then
            tar_handle:seek("cur", nread - header.size)
         end
      end

      if header.typeflag == "long name" then
         long_name = nullterm(file_data)
      elseif header.typeflag == "long link name" then
         long_link_name = nullterm(file_data)
      else
         if long_name then
            header.name = long_name
            long_name = nil
         end
         if long_link_name then
            header.name = long_link_name
            long_link_name = nil
         end
      end
      local pathname = dir.path(destdir, header.name)
      pathname = fs.absolute_name(pathname)
      if header.typeflag == "directory" then
         ok, err = make_dir(pathname)
         if not ok then
            break
         end
      elseif header.typeflag == "file" then
         local dirname = dir.dir_name(pathname)
         if dirname ~= "" then
            ok, err = make_dir(dirname)
            if not ok then
               break
            end
         end
         local file_handle
         file_handle, err = io.open(pathname, "wb")
         if not file_handle then
            ok = nil
            break
         end
         file_handle:write(file_data)
         file_handle:close()
         fs.set_time(pathname, header.mtime)
         if header.mode:match("[75]") then
            fs.set_permissions(pathname, "exec", "all")
         else
            fs.set_permissions(pathname, "read", "all")
         end
      end
      --[[
      for k,v in pairs(header) do
         util.printout("[\""..tostring(k).."\"] = "..(type(v)=="number" and v or "\""..v:gsub("%z", "\\0").."\""))
      end
      util.printout()
      --]]
   end
   tar_handle:close()
   return ok, err
end

return tar
