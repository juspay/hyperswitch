--- Patch utility to apply unified diffs.
--
-- http://lua-users.org/wiki/LuaPatch
--
-- (c) 2008 David Manura, Licensed under the same terms as Lua (MIT license).
-- Code is heavily based on the Python-based patch.py version 8.06-1
--   Copyright (c) 2008 rainforce.org, MIT License
--   Project home: http://code.google.com/p/python-patch/ .
--   Version 0.1

local patch = {}

local fs = require("luarocks.fs")
local fun = require("luarocks.fun")

local io = io
local os = os
local string = string
local table = table
local format = string.format

-- logging
local debugmode = false
local function debug(_) end
local function info(_) end
local function warning(s) io.stderr:write(s .. '\n') end

-- Returns boolean whether string s2 starts with string s.
local function startswith(s, s2)
  return s:sub(1, #s2) == s2
end

-- Returns boolean whether string s2 ends with string s.
local function endswith(s, s2)
  return #s >= #s2 and s:sub(#s-#s2+1) == s2
end

-- Returns string s after filtering out any new-line characters from end.
local function endlstrip(s)
  return s:gsub('[\r\n]+$', '')
end

-- Returns shallow copy of table t.
local function table_copy(t)
  local t2 = {}
  for k,v in pairs(t) do t2[k] = v end
  return t2
end

local function exists(filename)
  local fh = io.open(filename)
  local result = fh ~= nil
  if fh then fh:close() end
  return result
end
local function isfile() return true end --FIX?

local function string_as_file(s)
   return {
      at = 0,
      str = s,
      len = #s,
      eof = false,
      read = function(self, n)
         if self.eof then return nil end
         local chunk = self.str:sub(self.at, self.at + n - 1)
         self.at = self.at + n
         if self.at > self.len then
            self.eof = true
         end
         return chunk
      end,
      close = function(self)
         self.eof = true
      end,
   }
end

--
-- file_lines(f) is similar to f:lines() for file f.
-- The main difference is that read_lines includes
-- new-line character sequences ("\n", "\r\n", "\r"),
-- if any, at the end of each line.  Embedded "\0" are also handled.
-- Caution: The newline behavior can depend on whether f is opened
-- in binary or ASCII mode.
-- (file_lines - version 20080913)
--
local function file_lines(f)
  local CHUNK_SIZE = 1024
  local buffer = ""
  local pos_beg = 1
  return function()
    local pos, chars
    while 1 do
      pos, chars = buffer:match('()([\r\n].)', pos_beg)
      if pos or not f then
        break
      elseif f then
        local chunk = f:read(CHUNK_SIZE)
        if chunk then
          buffer = buffer:sub(pos_beg) .. chunk
          pos_beg = 1
        else
          f = nil
        end
      end
    end
    if not pos then
      pos = #buffer
    elseif chars == '\r\n' then
      pos = pos + 1
    end
    local line = buffer:sub(pos_beg, pos)
    pos_beg = pos + 1
    if #line > 0 then
      return line
    end
  end
end

local function match_linerange(line)
  local m1, m2, m3, m4 =      line:match("^@@ %-(%d+),(%d+) %+(%d+),(%d+)")
  if not m1 then m1, m3, m4 = line:match("^@@ %-(%d+) %+(%d+),(%d+)") end
  if not m1 then m1, m2, m3 = line:match("^@@ %-(%d+),(%d+) %+(%d+)") end
  if not m1 then m1, m3     = line:match("^@@ %-(%d+) %+(%d+)") end
  return m1, m2, m3, m4
end

local function match_epoch(str)
  return str:match("[^0-9]1969[^0-9]") or str:match("[^0-9]1970[^0-9]")
end

function patch.read_patch(filename, data)
  -- define possible file regions that will direct the parser flow
  local state = 'header'
    -- 'header'    - comments before the patch body
    -- 'filenames' - lines starting with --- and +++
    -- 'hunkhead'  - @@ -R +R @@ sequence
    -- 'hunkbody'
    -- 'hunkskip'  - skipping invalid hunk mode

  local all_ok = true
  local lineends = {lf=0, crlf=0, cr=0}
  local files = {source={}, target={}, epoch={}, hunks={}, fileends={}, hunkends={}}
  local nextfileno = 0
  local nexthunkno = 0    --: even if index starts with 0 user messages
                          --  number hunks from 1

  -- hunkinfo holds parsed values, hunkactual - calculated
  local hunkinfo = {
    startsrc=nil, linessrc=nil, starttgt=nil, linestgt=nil,
    invalid=false, text={}
  }
  local hunkactual = {linessrc=nil, linestgt=nil}

  info(format("reading patch %s", filename))

  local fp
  if data then
    fp = string_as_file(data)
  else
    fp = filename == '-' and io.stdin or assert(io.open(filename, "rb"))
  end
  local lineno = 0

  for line in file_lines(fp) do
    lineno = lineno + 1
    if state == 'header' then
      if startswith(line, "--- ") then
        state = 'filenames'
      end
      -- state is 'header' or 'filenames'
    end
    if state == 'hunkbody' then
      -- skip hunkskip and hunkbody code until definition of hunkhead read

      if line:match"^[\r\n]*$" then
          -- prepend space to empty lines to interpret them as context properly
          line = " " .. line
      end

      -- process line first
      if line:match"^[- +\\]" then
          -- gather stats about line endings
          local he = files.hunkends[nextfileno]
          if endswith(line, "\r\n") then
            he.crlf = he.crlf + 1
          elseif endswith(line, "\n") then
            he.lf = he.lf + 1
          elseif endswith(line, "\r") then
            he.cr = he.cr + 1
          end
          if startswith(line, "-") then
            hunkactual.linessrc = hunkactual.linessrc + 1
          elseif startswith(line, "+") then
            hunkactual.linestgt = hunkactual.linestgt + 1
          elseif startswith(line, "\\") then
            -- nothing
          else
            hunkactual.linessrc = hunkactual.linessrc + 1
            hunkactual.linestgt = hunkactual.linestgt + 1
          end
          table.insert(hunkinfo.text, line)
          -- todo: handle \ No newline cases
      else
          warning(format("invalid hunk no.%d at %d for target file %s",
                         nexthunkno, lineno, files.target[nextfileno]))
          -- add hunk status node
          table.insert(files.hunks[nextfileno], table_copy(hunkinfo))
          files.hunks[nextfileno][nexthunkno].invalid = true
          all_ok = false
          state = 'hunkskip'
      end

      -- check exit conditions
      if hunkactual.linessrc > hunkinfo.linessrc or
         hunkactual.linestgt > hunkinfo.linestgt
      then
          warning(format("extra hunk no.%d lines at %d for target %s",
                         nexthunkno, lineno, files.target[nextfileno]))
          -- add hunk status node
          table.insert(files.hunks[nextfileno], table_copy(hunkinfo))
          files.hunks[nextfileno][nexthunkno].invalid = true
          state = 'hunkskip'
      elseif hunkinfo.linessrc == hunkactual.linessrc and
             hunkinfo.linestgt == hunkactual.linestgt
      then
          table.insert(files.hunks[nextfileno], table_copy(hunkinfo))
          state = 'hunkskip'

          -- detect mixed window/unix line ends
          local ends = files.hunkends[nextfileno]
          if (ends.cr~=0 and 1 or 0) + (ends.crlf~=0 and 1 or 0) +
             (ends.lf~=0 and 1 or 0) > 1
          then
            warning(format("inconsistent line ends in patch hunks for %s",
                    files.source[nextfileno]))
          end
      end
      -- state is 'hunkbody' or 'hunkskip'
    end

    if state == 'hunkskip' then
      if match_linerange(line) then
        state = 'hunkhead'
      elseif startswith(line, "--- ") then
        state = 'filenames'
        if debugmode and #files.source > 0 then
            debug(format("- %2d hunks for %s", #files.hunks[nextfileno],
                         files.source[nextfileno]))
        end
      end
      -- state is 'hunkskip', 'hunkhead', or 'filenames'
    end
    local advance
    if state == 'filenames' then
      if startswith(line, "--- ") then
        if fun.contains(files.source, nextfileno) then
          all_ok = false
          warning(format("skipping invalid patch for %s",
                         files.source[nextfileno+1]))
          table.remove(files.source, nextfileno+1)
          -- double source filename line is encountered
          -- attempt to restart from this second line
        end
        -- Accept a space as a terminator, like GNU patch does.
        -- Breaks patches containing filenames with spaces...
        -- FIXME Figure out what does GNU patch do in those cases.
        local match, rest = line:match("^%-%-%- ([^ \t\r\n]+)(.*)")
        if not match then
          all_ok = false
          warning(format("skipping invalid filename at line %d", lineno+1))
          state = 'header'
        else
          if match_epoch(rest) then
            files.epoch[nextfileno + 1] = true
          end
          table.insert(files.source, match)
        end
      elseif not startswith(line, "+++ ") then
        if fun.contains(files.source, nextfileno) then
          all_ok = false
          warning(format("skipping invalid patch with no target for %s",
                         files.source[nextfileno+1]))
          table.remove(files.source, nextfileno+1)
        else
          -- this should be unreachable
          warning("skipping invalid target patch")
        end
        state = 'header'
      else
        if fun.contains(files.target, nextfileno) then
          all_ok = false
          warning(format("skipping invalid patch - double target at line %d",
                         lineno+1))
          table.remove(files.source, nextfileno+1)
          table.remove(files.target, nextfileno+1)
          nextfileno = nextfileno - 1
          -- double target filename line is encountered
          -- switch back to header state
          state = 'header'
        else
          -- Accept a space as a terminator, like GNU patch does.
          -- Breaks patches containing filenames with spaces...
          -- FIXME Figure out what does GNU patch do in those cases.
          local re_filename = "^%+%+%+ ([^ \t\r\n]+)(.*)$"
          local match, rest = line:match(re_filename)
          if not match then
            all_ok = false
            warning(format(
              "skipping invalid patch - no target filename at line %d",
              lineno+1))
            state = 'header'
          else
            table.insert(files.target, match)
            nextfileno = nextfileno + 1
            if match_epoch(rest) then
              files.epoch[nextfileno] = true
            end
            nexthunkno = 0
            table.insert(files.hunks, {})
            table.insert(files.hunkends, table_copy(lineends))
            table.insert(files.fileends, table_copy(lineends))
            state = 'hunkhead'
            advance = true
          end
        end
      end
      -- state is 'filenames', 'header', or ('hunkhead' with advance)
    end
    if not advance and state == 'hunkhead' then
      local m1, m2, m3, m4 = match_linerange(line)
      if not m1 then
        if not fun.contains(files.hunks, nextfileno-1) then
          all_ok = false
          warning(format("skipping invalid patch with no hunks for file %s",
                         files.target[nextfileno]))
        end
        state = 'header'
      else
        hunkinfo.startsrc = tonumber(m1)
        hunkinfo.linessrc = tonumber(m2 or 1)
        hunkinfo.starttgt = tonumber(m3)
        hunkinfo.linestgt = tonumber(m4 or 1)
        hunkinfo.invalid = false
        hunkinfo.text = {}

        hunkactual.linessrc = 0
        hunkactual.linestgt = 0

        state = 'hunkbody'
        nexthunkno = nexthunkno + 1
      end
      -- state is 'header' or 'hunkbody'
    end
  end
  if state ~= 'hunkskip' then
    warning(format("patch file incomplete - %s", filename))
    all_ok = false
    -- os.exit(?)
  else
    -- duplicated message when an eof is reached
    if debugmode and #files.source > 0 then
      debug(format("- %2d hunks for %s", #files.hunks[nextfileno],
                   files.source[nextfileno]))
    end
  end

  local sum = 0; for _,hset in ipairs(files.hunks) do sum = sum + #hset end
  info(format("total files: %d  total hunks: %d", #files.source, sum))
  fp:close()
  return files, all_ok
end

local function find_hunk(file, h, hno)
  for fuzz=0,2 do
    local lineno = h.startsrc
    for i=0,#file do
      local found = true
      local location = lineno
      for l, hline in ipairs(h.text) do
        if l > fuzz then
          -- todo: \ No newline at the end of file
          if startswith(hline, " ") or startswith(hline, "-") then
            local line = file[lineno]
            lineno = lineno + 1
            if not line or #line == 0 then
              found = false
              break
            end
            if endlstrip(line) ~= endlstrip(hline:sub(2)) then
              found = false
              break
            end
          end
        end
      end
      if found then
        local offset = location - h.startsrc - fuzz
        if offset ~= 0 then
          warning(format("Hunk %d found at offset %d%s...", hno, offset, fuzz == 0 and "" or format(" (fuzz %d)", fuzz)))
        end
        h.startsrc = location
        h.starttgt = h.starttgt + offset
        for _=1,fuzz do
           table.remove(h.text, 1)
           table.remove(h.text, #h.text)
        end
        return true
      end
      lineno = i
    end
  end
  return false
end

local function load_file(filename)
  local fp = assert(io.open(filename))
  local file = {}
  local readline = file_lines(fp)
  while true do
    local line = readline()
    if not line then break end
    table.insert(file, line)
  end
  fp:close()
  return file
end

local function find_hunks(file, hunks)
  for hno, h in ipairs(hunks) do
    find_hunk(file, h, hno)
  end
end

local function check_patched(file, hunks)
  local lineno = 1
  local ok, err = pcall(function()
    if #file == 0 then
      error('nomatch', 0)
    end
    for hno, h in ipairs(hunks) do
      -- skip to line just before hunk starts
      if #file < h.starttgt then
        error('nomatch', 0)
      end
      lineno = h.starttgt
      for _, hline in ipairs(h.text) do
        -- todo: \ No newline at the end of file
        if not startswith(hline, "-") and not startswith(hline, "\\") then
          local line = file[lineno]
          lineno = lineno + 1
          if #line == 0 then
            error('nomatch', 0)
          end
          if endlstrip(line) ~= endlstrip(hline:sub(2)) then
            warning(format("file is not patched - failed hunk: %d", hno))
            error('nomatch', 0)
          end
        end
      end
    end
  end)
  -- todo: display failed hunk, i.e. expected/found
  return err ~= 'nomatch'
end

local function patch_hunks(srcname, tgtname, hunks)
  local src = assert(io.open(srcname, "rb"))
  local tgt = assert(io.open(tgtname, "wb"))

  local src_readline = file_lines(src)

  -- todo: detect linefeeds early - in apply_files routine
  --       to handle cases when patch starts right from the first
  --       line and no lines are processed. At the moment substituted
  --       lineends may not be the same at the start and at the end
  --       of patching. Also issue a warning about mixed lineends

  local srclineno = 1
  local lineends = {['\n']=0, ['\r\n']=0, ['\r']=0}
  for hno, h in ipairs(hunks) do
    debug(format("processing hunk %d for file %s", hno, tgtname))
    -- skip to line just before hunk starts
    while srclineno < h.startsrc do
      local line = src_readline()
      -- Python 'U' mode works only with text files
      if endswith(line, "\r\n") then
        lineends["\r\n"] = lineends["\r\n"] + 1
      elseif endswith(line, "\n") then
        lineends["\n"] = lineends["\n"] + 1
      elseif endswith(line, "\r") then
        lineends["\r"] = lineends["\r"] + 1
      end
      tgt:write(line)
      srclineno = srclineno + 1
    end

    for _,hline in ipairs(h.text) do
      -- todo: check \ No newline at the end of file
      if startswith(hline, "-") or startswith(hline, "\\") then
        src_readline()
        srclineno = srclineno + 1
      else
        if not startswith(hline, "+") then
          src_readline()
          srclineno = srclineno + 1
        end
        local line2write = hline:sub(2)
        -- detect if line ends are consistent in source file
        local sum = 0
        for _,v in pairs(lineends) do if v > 0 then sum=sum+1 end end
        if sum == 1 then
          local newline
          for k,v in pairs(lineends) do if v ~= 0 then newline = k end end
          tgt:write(endlstrip(line2write) .. newline)
        else -- newlines are mixed or unknown
          tgt:write(line2write)
        end
      end
    end
  end
  for line in src_readline do
    tgt:write(line)
  end
  tgt:close()
  src:close()
  return true
end

local function strip_dirs(filename, strip)
  if strip == nil then return filename end
  for _=1,strip do
    filename=filename:gsub("^[^/]*/", "")
  end
  return filename
end

local function write_new_file(filename, hunk)
  local fh = io.open(filename, "wb")
  if not fh then return false end
  for _, hline in ipairs(hunk.text) do
    local c = hline:sub(1,1)
    if c ~= "+" and c ~= "-" and c ~= " " then
      return false, "malformed patch"
    end
    fh:write(hline:sub(2))
  end
  fh:close()
  return true
end

local function patch_file(source, target, epoch, hunks, strip, create_delete)
  local create_file = false
  if create_delete then
    local is_src_epoch = epoch and #hunks == 1 and hunks[1].startsrc == 0 and hunks[1].linessrc == 0
    if is_src_epoch or source == "/dev/null" then
      info(format("will create %s", target))
      create_file = true
    end
  end
  if create_file then
    return write_new_file(fs.absolute_name(strip_dirs(target, strip)), hunks[1])
  end
  source = strip_dirs(source, strip)
  local f2patch = source
  if not exists(f2patch) then
    f2patch = strip_dirs(target, strip)
    f2patch = fs.absolute_name(f2patch)
    if not exists(f2patch) then  --FIX:if f2patch nil
      warning(format("source/target file does not exist\n--- %s\n+++ %s",
              source, f2patch))
      return false
    end
  end
  if not isfile(f2patch) then
    warning(format("not a file - %s", f2patch))
    return false
  end

  source = f2patch

  -- validate before patching
  local file = load_file(source)
  local hunkno = 1
  local hunk = hunks[hunkno]
  local hunkfind = {}
  local validhunks = 0
  local canpatch = false
  local hunklineno
  if not file then
    return nil, "failed reading file " .. source
  end

  if create_delete then
    if epoch and #hunks == 1 and hunks[1].starttgt == 0 and hunks[1].linestgt == 0 then
      local ok = os.remove(source)
      if not ok then
        return false
      end
      info(format("successfully removed %s", source))
      return true
    end
  end

  find_hunks(file, hunks)

  local function process_line(line, lineno)
    if not hunk or lineno < hunk.startsrc then
      return false
    end
    if lineno == hunk.startsrc then
      hunkfind = {}
      for _,x in ipairs(hunk.text) do
        if x:sub(1,1) == ' ' or x:sub(1,1) == '-' then
          hunkfind[#hunkfind+1] = endlstrip(x:sub(2))
        end
      end
      hunklineno = 1

      -- todo \ No newline at end of file
    end
    -- check hunks in source file
    if lineno < hunk.startsrc + #hunkfind - 1 then
      if endlstrip(line) == hunkfind[hunklineno] then
        hunklineno = hunklineno + 1
      else
        debug(format("hunk no.%d doesn't match source file %s",
                     hunkno, source))
        -- file may be already patched, but check other hunks anyway
        hunkno = hunkno + 1
        if hunkno <= #hunks then
          hunk = hunks[hunkno]
          return false
        else
          return true
        end
      end
    end
    -- check if processed line is the last line
    if lineno == hunk.startsrc + #hunkfind - 1 then
      debug(format("file %s hunk no.%d -- is ready to be patched",
                   source, hunkno))
      hunkno = hunkno + 1
      validhunks = validhunks + 1
      if hunkno <= #hunks then
        hunk = hunks[hunkno]
      else
        if validhunks == #hunks then
          -- patch file
          canpatch = true
          return true
        end
      end
    end
    return false
  end

  local done = false
  for lineno, line in ipairs(file) do
    done = process_line(line, lineno)
    if done then
      break
    end
  end
  if not done then
    if hunkno <= #hunks and not create_file then
      warning(format("premature end of source file %s at hunk %d",
                     source, hunkno))
      return false
    end
  end
  if validhunks < #hunks then
    if check_patched(file, hunks) then
      warning(format("already patched  %s", source))
    elseif not create_file then
      warning(format("source file is different - %s", source))
      return false
    end
  end
  if not canpatch then
    return true
  end
  local backupname = source .. ".orig"
  if exists(backupname) then
    warning(format("can't backup original file to %s - aborting",
                   backupname))
    return false
  end
  local ok = os.rename(source, backupname)
  if not ok then
    warning(format("failed backing up %s when patching", source))
    return false
  end
  patch_hunks(backupname, source, hunks)
  info(format("successfully patched %s", source))
  os.remove(backupname)
  return true
end

function patch.apply_patch(the_patch, strip, create_delete)
  local all_ok = true
  local total = #the_patch.source
  for fileno, source in ipairs(the_patch.source) do
    local target = the_patch.target[fileno]
    local hunks = the_patch.hunks[fileno]
    local epoch = the_patch.epoch[fileno]
    info(format("processing %d/%d:\t %s", fileno, total, source))
    local ok = patch_file(source, target, epoch, hunks, strip, create_delete)
    all_ok = all_ok and ok
  end
  -- todo: check for premature eof
  return all_ok
end

return patch
