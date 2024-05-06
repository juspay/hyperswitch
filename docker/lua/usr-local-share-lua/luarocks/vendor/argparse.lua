-- The MIT License (MIT)

-- Copyright (c) 2013 - 2018 Peter Melnichenko
--                      2019 Paul Ouellette

-- Permission is hereby granted, free of charge, to any person obtaining a copy of
-- this software and associated documentation files (the "Software"), to deal in
-- the Software without restriction, including without limitation the rights to
-- use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
-- the Software, and to permit persons to whom the Software is furnished to do so,
-- subject to the following conditions:

-- The above copyright notice and this permission notice shall be included in all
-- copies or substantial portions of the Software.

-- THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
-- IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
-- FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
-- COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
-- IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
-- CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

local function deep_update(t1, t2)
   for k, v in pairs(t2) do
      if type(v) == "table" then
         v = deep_update({}, v)
      end

      t1[k] = v
   end

   return t1
end

-- A property is a tuple {name, callback}.
-- properties.args is number of properties that can be set as arguments
-- when calling an object.
local function class(prototype, properties, parent)
   -- Class is the metatable of its instances.
   local cl = {}
   cl.__index = cl

   if parent then
      cl.__prototype = deep_update(deep_update({}, parent.__prototype), prototype)
   else
      cl.__prototype = prototype
   end

   if properties then
      local names = {}

      -- Create setter methods and fill set of property names.
      for _, property in ipairs(properties) do
         local name, callback = property[1], property[2]

         cl[name] = function(self, value)
            if not callback(self, value) then
               self["_" .. name] = value
            end

            return self
         end

         names[name] = true
      end

      function cl.__call(self, ...)
         -- When calling an object, if the first argument is a table,
         -- interpret keys as property names, else delegate arguments
         -- to corresponding setters in order.
         if type((...)) == "table" then
            for name, value in pairs((...)) do
               if names[name] then
                  self[name](self, value)
               end
            end
         else
            local nargs = select("#", ...)

            for i, property in ipairs(properties) do
               if i > nargs or i > properties.args then
                  break
               end

               local arg = select(i, ...)

               if arg ~= nil then
                  self[property[1]](self, arg)
               end
            end
         end

         return self
      end
   end

   -- If indexing class fails, fallback to its parent.
   local class_metatable = {}
   class_metatable.__index = parent

   function class_metatable.__call(self, ...)
      -- Calling a class returns its instance.
      -- Arguments are delegated to the instance.
      local object = deep_update({}, self.__prototype)
      setmetatable(object, self)
      return object(...)
   end

   return setmetatable(cl, class_metatable)
end

local function typecheck(name, types, value)
   for _, type_ in ipairs(types) do
      if type(value) == type_ then
         return true
      end
   end

   error(("bad property '%s' (%s expected, got %s)"):format(name, table.concat(types, " or "), type(value)))
end

local function typechecked(name, ...)
   local types = {...}
   return {name, function(_, value) typecheck(name, types, value) end}
end

local multiname = {"name", function(self, value)
   typecheck("name", {"string"}, value)

   for alias in value:gmatch("%S+") do
      self._name = self._name or alias
      table.insert(self._aliases, alias)
      table.insert(self._public_aliases, alias)
      -- If alias contains '_', accept '-' also.
      if alias:find("_", 1, true) then
         table.insert(self._aliases, (alias:gsub("_", "-")))
      end
   end

   -- Do not set _name as with other properties.
   return true
end}

local multiname_hidden = {"hidden_name", function(self, value)
   typecheck("hidden_name", {"string"}, value)

   for alias in value:gmatch("%S+") do
      table.insert(self._aliases, alias)
      if alias:find("_", 1, true) then
         table.insert(self._aliases, (alias:gsub("_", "-")))
      end
   end

   return true
end}

local function parse_boundaries(str)
   if tonumber(str) then
      return tonumber(str), tonumber(str)
   end

   if str == "*" then
      return 0, math.huge
   end

   if str == "+" then
      return 1, math.huge
   end

   if str == "?" then
      return 0, 1
   end

   if str:match "^%d+%-%d+$" then
      local min, max = str:match "^(%d+)%-(%d+)$"
      return tonumber(min), tonumber(max)
   end

   if str:match "^%d+%+$" then
      local min = str:match "^(%d+)%+$"
      return tonumber(min), math.huge
   end
end

local function boundaries(name)
   return {name, function(self, value)
      typecheck(name, {"number", "string"}, value)

      local min, max = parse_boundaries(value)

      if not min then
         error(("bad property '%s'"):format(name))
      end

      self["_min" .. name], self["_max" .. name] = min, max
   end}
end

local actions = {}

local option_action = {"action", function(_, value)
   typecheck("action", {"function", "string"}, value)

   if type(value) == "string" and not actions[value] then
      error(("unknown action '%s'"):format(value))
   end
end}

local option_init = {"init", function(self)
   self._has_init = true
end}

local option_default = {"default", function(self, value)
   if type(value) ~= "string" then
      self._init = value
      self._has_init = true
      return true
   end
end}

local add_help = {"add_help", function(self, value)
   typecheck("add_help", {"boolean", "string", "table"}, value)

   if self._help_option_idx then
      table.remove(self._options, self._help_option_idx)
      self._help_option_idx = nil
   end

   if value then
      local help = self:flag()
         :description "Show this help message and exit."
         :action(function()
            print(self:get_help())
            os.exit(0)
         end)

      if value ~= true then
         help = help(value)
      end

      if not help._name then
         help "-h" "--help"
      end

      self._help_option_idx = #self._options
   end
end}

local Parser = class({
   _arguments = {},
   _options = {},
   _commands = {},
   _mutexes = {},
   _groups = {},
   _require_command = true,
   _handle_options = true
}, {
   args = 3,
   typechecked("name", "string"),
   typechecked("description", "string"),
   typechecked("epilog", "string"),
   typechecked("usage", "string"),
   typechecked("help", "string"),
   typechecked("require_command", "boolean"),
   typechecked("handle_options", "boolean"),
   typechecked("action", "function"),
   typechecked("command_target", "string"),
   typechecked("help_vertical_space", "number"),
   typechecked("usage_margin", "number"),
   typechecked("usage_max_width", "number"),
   typechecked("help_usage_margin", "number"),
   typechecked("help_description_margin", "number"),
   typechecked("help_max_width", "number"),
   add_help
})

local Command = class({
   _aliases = {},
   _public_aliases = {}
}, {
   args = 3,
   multiname,
   typechecked("description", "string"),
   typechecked("epilog", "string"),
   multiname_hidden,
   typechecked("summary", "string"),
   typechecked("target", "string"),
   typechecked("usage", "string"),
   typechecked("help", "string"),
   typechecked("require_command", "boolean"),
   typechecked("handle_options", "boolean"),
   typechecked("action", "function"),
   typechecked("command_target", "string"),
   typechecked("help_vertical_space", "number"),
   typechecked("usage_margin", "number"),
   typechecked("usage_max_width", "number"),
   typechecked("help_usage_margin", "number"),
   typechecked("help_description_margin", "number"),
   typechecked("help_max_width", "number"),
   typechecked("hidden", "boolean"),
   add_help
}, Parser)

local Argument = class({
   _minargs = 1,
   _maxargs = 1,
   _mincount = 1,
   _maxcount = 1,
   _defmode = "unused",
   _show_default = true
}, {
   args = 5,
   typechecked("name", "string"),
   typechecked("description", "string"),
   option_default,
   typechecked("convert", "function", "table"),
   boundaries("args"),
   typechecked("target", "string"),
   typechecked("defmode", "string"),
   typechecked("show_default", "boolean"),
   typechecked("argname", "string", "table"),
   typechecked("choices", "table"),
   typechecked("hidden", "boolean"),
   option_action,
   option_init
})

local Option = class({
   _aliases = {},
   _public_aliases = {},
   _mincount = 0,
   _overwrite = true
}, {
   args = 6,
   multiname,
   typechecked("description", "string"),
   option_default,
   typechecked("convert", "function", "table"),
   boundaries("args"),
   boundaries("count"),
   multiname_hidden,
   typechecked("target", "string"),
   typechecked("defmode", "string"),
   typechecked("show_default", "boolean"),
   typechecked("overwrite", "boolean"),
   typechecked("argname", "string", "table"),
   typechecked("choices", "table"),
   typechecked("hidden", "boolean"),
   option_action,
   option_init
}, Argument)

function Parser:_inherit_property(name, default)
   local element = self

   while true do
      local value = element["_" .. name]

      if value ~= nil then
         return value
      end

      if not element._parent then
         return default
      end

      element = element._parent
   end
end

function Argument:_get_argument_list()
   local buf = {}
   local i = 1

   while i <= math.min(self._minargs, 3) do
      local argname = self:_get_argname(i)

      if self._default and self._defmode:find "a" then
         argname = "[" .. argname .. "]"
      end

      table.insert(buf, argname)
      i = i+1
   end

   while i <= math.min(self._maxargs, 3) do
      table.insert(buf, "[" .. self:_get_argname(i) .. "]")
      i = i+1

      if self._maxargs == math.huge then
         break
      end
   end

   if i < self._maxargs then
      table.insert(buf, "...")
   end

   return buf
end

function Argument:_get_usage()
   local usage = table.concat(self:_get_argument_list(), " ")

   if self._default and self._defmode:find "u" then
      if self._maxargs > 1 or (self._minargs == 1 and not self._defmode:find "a") then
         usage = "[" .. usage .. "]"
      end
   end

   return usage
end

function actions.store_true(result, target)
   result[target] = true
end

function actions.store_false(result, target)
   result[target] = false
end

function actions.store(result, target, argument)
   result[target] = argument
end

function actions.count(result, target, _, overwrite)
   if not overwrite then
      result[target] = result[target] + 1
   end
end

function actions.append(result, target, argument, overwrite)
   result[target] = result[target] or {}
   table.insert(result[target], argument)

   if overwrite then
      table.remove(result[target], 1)
   end
end

function actions.concat(result, target, arguments, overwrite)
   if overwrite then
      error("'concat' action can't handle too many invocations")
   end

   result[target] = result[target] or {}

   for _, argument in ipairs(arguments) do
      table.insert(result[target], argument)
   end
end

function Argument:_get_action()
   local action, init

   if self._maxcount == 1 then
      if self._maxargs == 0 then
         action, init = "store_true", nil
      else
         action, init = "store", nil
      end
   else
      if self._maxargs == 0 then
         action, init = "count", 0
      else
         action, init = "append", {}
      end
   end

   if self._action then
      action = self._action
   end

   if self._has_init then
      init = self._init
   end

   if type(action) == "string" then
      action = actions[action]
   end

   return action, init
end

-- Returns placeholder for `narg`-th argument.
function Argument:_get_argname(narg)
   local argname = self._argname or self:_get_default_argname()

   if type(argname) == "table" then
      return argname[narg]
   else
      return argname
   end
end

function Argument:_get_choices_list()
   return "{" .. table.concat(self._choices, ",") .. "}"
end

function Argument:_get_default_argname()
   if self._choices then
      return self:_get_choices_list()
   else
      return "<" .. self._name .. ">"
   end
end

function Option:_get_default_argname()
   if self._choices then
      return self:_get_choices_list()
   else
      return "<" .. self:_get_default_target() .. ">"
   end
end

-- Returns labels to be shown in the help message.
function Argument:_get_label_lines()
   if self._choices then
      return {self:_get_choices_list()}
   else
      return {self._name}
   end
end

function Option:_get_label_lines()
   local argument_list = self:_get_argument_list()

   if #argument_list == 0 then
      -- Don't put aliases for simple flags like `-h` on different lines.
      return {table.concat(self._public_aliases, ", ")}
   end

   local longest_alias_length = -1

   for _, alias in ipairs(self._public_aliases) do
      longest_alias_length = math.max(longest_alias_length, #alias)
   end

   local argument_list_repr = table.concat(argument_list, " ")
   local lines = {}

   for i, alias in ipairs(self._public_aliases) do
      local line = (" "):rep(longest_alias_length - #alias) .. alias .. " " .. argument_list_repr

      if i ~= #self._public_aliases then
         line = line .. ","
      end

      table.insert(lines, line)
   end

   return lines
end

function Command:_get_label_lines()
   return {table.concat(self._public_aliases, ", ")}
end

function Argument:_get_description()
   if self._default and self._show_default then
      if self._description then
         return ("%s (default: %s)"):format(self._description, self._default)
      else
         return ("default: %s"):format(self._default)
      end
   else
      return self._description or ""
   end
end

function Command:_get_description()
   return self._summary or self._description or ""
end

function Option:_get_usage()
   local usage = self:_get_argument_list()
   table.insert(usage, 1, self._name)
   usage = table.concat(usage, " ")

   if self._mincount == 0 or self._default then
      usage = "[" .. usage .. "]"
   end

   return usage
end

function Argument:_get_default_target()
   return self._name
end

function Option:_get_default_target()
   local res

   for _, alias in ipairs(self._public_aliases) do
      if alias:sub(1, 1) == alias:sub(2, 2) then
         res = alias:sub(3)
         break
      end
   end

   res = res or self._name:sub(2)
   return (res:gsub("-", "_"))
end

function Option:_is_vararg()
   return self._maxargs ~= self._minargs
end

function Parser:_get_fullname(exclude_root)
   local parent = self._parent
   if exclude_root and not parent then
      return ""
   end
   local buf = {self._name}

   while parent do
      if not exclude_root or parent._parent then
         table.insert(buf, 1, parent._name)
      end
      parent = parent._parent
   end

   return table.concat(buf, " ")
end

function Parser:_update_charset(charset)
   charset = charset or {}

   for _, command in ipairs(self._commands) do
      command:_update_charset(charset)
   end

   for _, option in ipairs(self._options) do
      for _, alias in ipairs(option._aliases) do
         charset[alias:sub(1, 1)] = true
      end
   end

   return charset
end

function Parser:argument(...)
   local argument = Argument(...)
   table.insert(self._arguments, argument)
   return argument
end

function Parser:option(...)
   local option = Option(...)
   table.insert(self._options, option)
   return option
end

function Parser:flag(...)
   return self:option():args(0)(...)
end

function Parser:command(...)
   local command = Command():add_help(true)(...)
   command._parent = self
   table.insert(self._commands, command)
   return command
end

function Parser:mutex(...)
   local elements = {...}

   for i, element in ipairs(elements) do
      local mt = getmetatable(element)
      assert(mt == Option or mt == Argument, ("bad argument #%d to 'mutex' (Option or Argument expected)"):format(i))
   end

   table.insert(self._mutexes, elements)
   return self
end

function Parser:group(name, ...)
   assert(type(name) == "string", ("bad argument #1 to 'group' (string expected, got %s)"):format(type(name)))

   local group = {name = name, ...}

   for i, element in ipairs(group) do
      local mt = getmetatable(element)
      assert(mt == Option or mt == Argument or mt == Command,
         ("bad argument #%d to 'group' (Option or Argument or Command expected)"):format(i + 1))
   end

   table.insert(self._groups, group)
   return self
end

local usage_welcome = "Usage: "

function Parser:get_usage()
   if self._usage then
      return self._usage
   end

   local usage_margin = self:_inherit_property("usage_margin", #usage_welcome)
   local max_usage_width = self:_inherit_property("usage_max_width", 70)
   local lines = {usage_welcome .. self:_get_fullname()}

   local function add(s)
      if #lines[#lines]+1+#s <= max_usage_width then
         lines[#lines] = lines[#lines] .. " " .. s
      else
         lines[#lines+1] = (" "):rep(usage_margin) .. s
      end
   end

   -- Normally options are before positional arguments in usage messages.
   -- However, vararg options should be after, because they can't be reliable used
   -- before a positional argument.
   -- Mutexes come into play, too, and are shown as soon as possible.
   -- Overall, output usages in the following order:
   -- 1. Mutexes that don't have positional arguments or vararg options.
   -- 2. Options that are not in any mutexes and are not vararg.
   -- 3. Positional arguments - on their own or as a part of a mutex.
   -- 4. Remaining mutexes.
   -- 5. Remaining options.

   local elements_in_mutexes = {}
   local added_elements = {}
   local added_mutexes = {}
   local argument_to_mutexes = {}

   local function add_mutex(mutex, main_argument)
      if added_mutexes[mutex] then
         return
      end

      added_mutexes[mutex] = true
      local buf = {}

      for _, element in ipairs(mutex) do
         if not element._hidden and not added_elements[element] then
            if getmetatable(element) == Option or element == main_argument then
               table.insert(buf, element:_get_usage())
               added_elements[element] = true
            end
         end
      end

      if #buf == 1 then
         add(buf[1])
      elseif #buf > 1 then
         add("(" .. table.concat(buf, " | ") .. ")")
      end
   end

   local function add_element(element)
      if not element._hidden and not added_elements[element] then
         add(element:_get_usage())
         added_elements[element] = true
      end
   end

   for _, mutex in ipairs(self._mutexes) do
      local is_vararg = false
      local has_argument = false

      for _, element in ipairs(mutex) do
         if getmetatable(element) == Option then
            if element:_is_vararg() then
               is_vararg = true
            end
         else
            has_argument = true
            argument_to_mutexes[element] = argument_to_mutexes[element] or {}
            table.insert(argument_to_mutexes[element], mutex)
         end

         elements_in_mutexes[element] = true
      end

      if not is_vararg and not has_argument then
         add_mutex(mutex)
      end
   end

   for _, option in ipairs(self._options) do
      if not elements_in_mutexes[option] and not option:_is_vararg() then
         add_element(option)
      end
   end

   -- Add usages for positional arguments, together with one mutex containing them, if they are in a mutex.
   for _, argument in ipairs(self._arguments) do
      -- Pick a mutex as a part of which to show this argument, take the first one that's still available.
      local mutex

      if elements_in_mutexes[argument] then
         for _, argument_mutex in ipairs(argument_to_mutexes[argument]) do
            if not added_mutexes[argument_mutex] then
               mutex = argument_mutex
            end
         end
      end

      if mutex then
         add_mutex(mutex, argument)
      else
         add_element(argument)
      end
   end

   for _, mutex in ipairs(self._mutexes) do
      add_mutex(mutex)
   end

   for _, option in ipairs(self._options) do
      add_element(option)
   end

   if #self._commands > 0 then
      if self._require_command then
         add("<command>")
      else
         add("[<command>]")
      end

      add("...")
   end

   return table.concat(lines, "\n")
end

local function split_lines(s)
   if s == "" then
      return {}
   end

   local lines = {}

   if s:sub(-1) ~= "\n" then
      s = s .. "\n"
   end

   for line in s:gmatch("([^\n]*)\n") do
      table.insert(lines, line)
   end

   return lines
end

local function autowrap_line(line, max_length)
   -- Algorithm for splitting lines is simple and greedy.
   local result_lines = {}

   -- Preserve original indentation of the line, put this at the beginning of each result line.
   -- If the first word looks like a list marker ('*', '+', or '-'), add spaces so that starts
   -- of the second and the following lines vertically align with the start of the second word.
   local indentation = line:match("^ *")

   if line:find("^ *[%*%+%-]") then
      indentation = indentation .. " " .. line:match("^ *[%*%+%-]( *)")
   end

   -- Parts of the last line being assembled.
   local line_parts = {}

   -- Length of the current line.
   local line_length = 0

   -- Index of the next character to consider.
   local index = 1

   while true do
      local word_start, word_finish, word = line:find("([^ ]+)", index)

      if not word_start then
         -- Ignore trailing spaces, if any.
         break
      end

      local preceding_spaces = line:sub(index, word_start - 1)
      index = word_finish + 1

      if (#line_parts == 0) or (line_length + #preceding_spaces + #word <= max_length) then
         -- Either this is the very first word or it fits as an addition to the current line, add it.
         table.insert(line_parts, preceding_spaces) -- For the very first word this adds the indentation.
         table.insert(line_parts, word)
         line_length = line_length + #preceding_spaces + #word
      else
         -- Does not fit, finish current line and put the word into a new one.
         table.insert(result_lines, table.concat(line_parts))
         line_parts = {indentation, word}
         line_length = #indentation + #word
      end
   end

   if #line_parts > 0 then
      table.insert(result_lines, table.concat(line_parts))
   end

   if #result_lines == 0 then
      -- Preserve empty lines.
      result_lines[1] = ""
   end

   return result_lines
end

-- Automatically wraps lines within given array,
-- attempting to limit line length to `max_length`.
-- Existing line splits are preserved.
local function autowrap(lines, max_length)
   local result_lines = {}

   for _, line in ipairs(lines) do
      local autowrapped_lines = autowrap_line(line, max_length)

      for _, autowrapped_line in ipairs(autowrapped_lines) do
         table.insert(result_lines, autowrapped_line)
      end
   end

   return result_lines
end

function Parser:_get_element_help(element)
   local label_lines = element:_get_label_lines()
   local description_lines = split_lines(element:_get_description())

   local result_lines = {}

   -- All label lines should have the same length (except the last one, it has no comma).
   -- If too long, start description after all the label lines.
   -- Otherwise, combine label and description lines.

   local usage_margin_len = self:_inherit_property("help_usage_margin", 3)
   local usage_margin = (" "):rep(usage_margin_len)
   local description_margin_len = self:_inherit_property("help_description_margin", 25)
   local description_margin = (" "):rep(description_margin_len)

   local help_max_width = self:_inherit_property("help_max_width")

   if help_max_width then
      local description_max_width = math.max(help_max_width - description_margin_len, 10)
      description_lines = autowrap(description_lines, description_max_width)
   end

   if #label_lines[1] >= (description_margin_len - usage_margin_len) then
      for _, label_line in ipairs(label_lines) do
         table.insert(result_lines, usage_margin .. label_line)
      end

      for _, description_line in ipairs(description_lines) do
         table.insert(result_lines, description_margin .. description_line)
      end
   else
      for i = 1, math.max(#label_lines, #description_lines) do
         local label_line = label_lines[i]
         local description_line = description_lines[i]

         local line = ""

         if label_line then
            line = usage_margin .. label_line
         end

         if description_line and description_line ~= "" then
            line = line .. (" "):rep(description_margin_len - #line) .. description_line
         end

         table.insert(result_lines, line)
      end
   end

   return table.concat(result_lines, "\n")
end

local function get_group_types(group)
   local types = {}

   for _, element in ipairs(group) do
      types[getmetatable(element)] = true
   end

   return types
end

function Parser:_add_group_help(blocks, added_elements, label, elements)
   local buf = {label}

   for _, element in ipairs(elements) do
      if not element._hidden and not added_elements[element] then
         added_elements[element] = true
         table.insert(buf, self:_get_element_help(element))
      end
   end

   if #buf > 1 then
      table.insert(blocks, table.concat(buf, ("\n"):rep(self:_inherit_property("help_vertical_space", 0) + 1)))
   end
end

function Parser:get_help()
   if self._help then
      return self._help
   end

   local blocks = {self:get_usage()}

   local help_max_width = self:_inherit_property("help_max_width")

   if self._description then
      local description = self._description

      if help_max_width then
         description = table.concat(autowrap(split_lines(description), help_max_width), "\n")
      end

      table.insert(blocks, description)
   end

   -- 1. Put groups containing arguments first, then other arguments.
   -- 2. Put remaining groups containing options, then other options.
   -- 3. Put remaining groups containing commands, then other commands.
   -- Assume that an element can't be in several groups.
   local groups_by_type = {
      [Argument] = {},
      [Option] = {},
      [Command] = {}
   }

   for _, group in ipairs(self._groups) do
      local group_types = get_group_types(group)

      for _, mt in ipairs({Argument, Option, Command}) do
         if group_types[mt] then
            table.insert(groups_by_type[mt], group)
            break
         end
      end
   end

   local default_groups = {
      {name = "Arguments", type = Argument, elements = self._arguments},
      {name = "Options", type = Option, elements = self._options},
      {name = "Commands", type = Command, elements = self._commands}
   }

   local added_elements = {}

   for _, default_group in ipairs(default_groups) do
      local type_groups = groups_by_type[default_group.type]

      for _, group in ipairs(type_groups) do
         self:_add_group_help(blocks, added_elements, group.name .. ":", group)
      end

      local default_label = default_group.name .. ":"

      if #type_groups > 0 then
         default_label = "Other " .. default_label:gsub("^.", string.lower)
      end

      self:_add_group_help(blocks, added_elements, default_label, default_group.elements)
   end

   if self._epilog then
      local epilog = self._epilog

      if help_max_width then
         epilog = table.concat(autowrap(split_lines(epilog), help_max_width), "\n")
      end

      table.insert(blocks, epilog)
   end

   return table.concat(blocks, "\n\n")
end

function Parser:add_help_command(value)
   if value then
      assert(type(value) == "string" or type(value) == "table",
         ("bad argument #1 to 'add_help_command' (string or table expected, got %s)"):format(type(value)))
   end

   local help = self:command()
      :description "Show help for commands."
   help:argument "command"
      :description "The command to show help for."
      :args "?"
      :action(function(_, _, cmd)
         if not cmd then
            print(self:get_help())
            os.exit(0)
         else
            for _, command in ipairs(self._commands) do
               for _, alias in ipairs(command._aliases) do
                  if alias == cmd then
                     print(command:get_help())
                     os.exit(0)
                  end
               end
            end
         end
         help:error(("unknown command '%s'"):format(cmd))
      end)

   if value then
      help = help(value)
   end

   if not help._name then
      help "help"
   end

   help._is_help_command = true
   return self
end

function Parser:_is_shell_safe()
   if self._basename then
      if self._basename:find("[^%w_%-%+%.]") then
         return false
      end
   else
      for _, alias in ipairs(self._aliases) do
         if alias:find("[^%w_%-%+%.]") then
            return false
         end
      end
   end
   for _, option in ipairs(self._options) do
      for _, alias in ipairs(option._aliases) do
         if alias:find("[^%w_%-%+%.]") then
            return false
         end
      end
      if option._choices then
         for _, choice in ipairs(option._choices) do
            if choice:find("[%s'\"]") then
               return false
            end
         end
      end
   end
   for _, argument in ipairs(self._arguments) do
      if argument._choices then
         for _, choice in ipairs(argument._choices) do
            if choice:find("[%s'\"]") then
               return false
            end
         end
      end
   end
   for _, command in ipairs(self._commands) do
      if not command:_is_shell_safe() then
         return false
      end
   end
   return true
end

function Parser:add_complete(value)
   if value then
      assert(type(value) == "string" or type(value) == "table",
         ("bad argument #1 to 'add_complete' (string or table expected, got %s)"):format(type(value)))
   end

   local complete = self:option()
      :description "Output a shell completion script for the specified shell."
      :args(1)
      :choices {"bash", "zsh", "fish"}
      :action(function(_, _, shell)
         io.write(self["get_" .. shell .. "_complete"](self))
         os.exit(0)
      end)

   if value then
      complete = complete(value)
   end

   if not complete._name then
      complete "--completion"
   end

   return self
end

function Parser:add_complete_command(value)
   if value then
      assert(type(value) == "string" or type(value) == "table",
         ("bad argument #1 to 'add_complete_command' (string or table expected, got %s)"):format(type(value)))
   end

   local complete = self:command()
      :description "Output a shell completion script."
   complete:argument "shell"
      :description "The shell to output a completion script for."
      :choices {"bash", "zsh", "fish"}
      :action(function(_, _, shell)
         io.write(self["get_" .. shell .. "_complete"](self))
         os.exit(0)
      end)

   if value then
      complete = complete(value)
   end

   if not complete._name then
      complete "completion"
   end

   return self
end

local function base_name(pathname)
   return pathname:gsub("[/\\]*$", ""):match(".*[/\\]([^/\\]*)") or pathname
end

local function get_short_description(element)
   local short = element:_get_description():match("^(.-)%.%s")
   return short or element:_get_description():match("^(.-)%.?$")
end

function Parser:_get_options()
   local options = {}
   for _, option in ipairs(self._options) do
      for _, alias in ipairs(option._aliases) do
         table.insert(options, alias)
      end
   end
   return table.concat(options, " ")
end

function Parser:_get_commands()
   local commands = {}
   for _, command in ipairs(self._commands) do
      for _, alias in ipairs(command._aliases) do
         table.insert(commands, alias)
      end
   end
   return table.concat(commands, " ")
end

function Parser:_bash_option_args(buf, indent)
   local opts = {}
   for _, option in ipairs(self._options) do
      if option._choices or option._minargs > 0 then
         local compreply
         if option._choices then
            compreply = 'COMPREPLY=($(compgen -W "' .. table.concat(option._choices, " ") .. '" -- "$cur"))'
         else
            compreply = 'COMPREPLY=($(compgen -f -- "$cur"))'
         end
         table.insert(opts, (" "):rep(indent + 4) .. table.concat(option._aliases, "|") .. ")")
         table.insert(opts, (" "):rep(indent + 8) .. compreply)
         table.insert(opts, (" "):rep(indent + 8) .. "return 0")
         table.insert(opts, (" "):rep(indent + 8) .. ";;")
      end
   end

   if #opts > 0 then
      table.insert(buf, (" "):rep(indent) .. 'case "$prev" in')
      table.insert(buf, table.concat(opts, "\n"))
      table.insert(buf, (" "):rep(indent) .. "esac\n")
   end
end

function Parser:_bash_get_cmd(buf, indent)
   if #self._commands == 0 then
      return
   end

   table.insert(buf, (" "):rep(indent) .. 'args=("${args[@]:1}")')
   table.insert(buf, (" "):rep(indent) .. 'for arg in "${args[@]}"; do')
   table.insert(buf, (" "):rep(indent + 4) .. 'case "$arg" in')

   for _, command in ipairs(self._commands) do
      table.insert(buf, (" "):rep(indent + 8) .. table.concat(command._aliases, "|") .. ")")
      if self._parent then
         table.insert(buf, (" "):rep(indent + 12) .. 'cmd="$cmd ' .. command._name .. '"')
      else
         table.insert(buf, (" "):rep(indent + 12) .. 'cmd="' .. command._name .. '"')
      end
      table.insert(buf, (" "):rep(indent + 12) .. 'opts="$opts ' .. command:_get_options() .. '"')
      command:_bash_get_cmd(buf, indent + 12)
      table.insert(buf, (" "):rep(indent + 12) .. "break")
      table.insert(buf, (" "):rep(indent + 12) .. ";;")
   end

   table.insert(buf, (" "):rep(indent + 4) .. "esac")
   table.insert(buf, (" "):rep(indent) .. "done")
end

function Parser:_bash_cmd_completions(buf)
   local cmd_buf = {}
   if self._parent then
      self:_bash_option_args(cmd_buf, 12)
   end
   if #self._commands > 0 then
      table.insert(cmd_buf, (" "):rep(12) .. 'COMPREPLY=($(compgen -W "' .. self:_get_commands() .. '" -- "$cur"))')
   elseif self._is_help_command then
      table.insert(cmd_buf, (" "):rep(12)
         .. 'COMPREPLY=($(compgen -W "'
         .. self._parent:_get_commands()
         .. '" -- "$cur"))')
   end
   if #cmd_buf > 0 then
      table.insert(buf, (" "):rep(8) .. "'" .. self:_get_fullname(true) .. "')")
      table.insert(buf, table.concat(cmd_buf, "\n"))
      table.insert(buf, (" "):rep(12) .. ";;")
   end

   for _, command in ipairs(self._commands) do
      command:_bash_cmd_completions(buf)
   end
end

function Parser:get_bash_complete()
   self._basename = base_name(self._name)
   assert(self:_is_shell_safe())
   local buf = {([[
_%s() {
    local IFS=$' \t\n'
    local args cur prev cmd opts arg
    args=("${COMP_WORDS[@]}")
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="%s"
]]):format(self._basename, self:_get_options())}

   self:_bash_option_args(buf, 4)
   self:_bash_get_cmd(buf, 4)
   if #self._commands > 0 then
      table.insert(buf, "")
      table.insert(buf, (" "):rep(4) .. 'case "$cmd" in')
      self:_bash_cmd_completions(buf)
      table.insert(buf, (" "):rep(4) .. "esac\n")
   end

   table.insert(buf, ([=[
    if [[ "$cur" = -* ]]; then
        COMPREPLY=($(compgen -W "$opts" -- "$cur"))
    fi
}

complete -F _%s -o bashdefault -o default %s
]=]):format(self._basename, self._basename))

   return table.concat(buf, "\n")
end

function Parser:_zsh_arguments(buf, cmd_name, indent)
   if self._parent then
      table.insert(buf, (" "):rep(indent) .. "options=(")
      table.insert(buf, (" "):rep(indent + 2) .. "$options")
   else
      table.insert(buf, (" "):rep(indent) .. "local -a options=(")
   end

   for _, option in ipairs(self._options) do
      local line = {}
      if #option._aliases > 1 then
         if option._maxcount > 1 then
            table.insert(line, '"*"')
         end
         table.insert(line, "{" .. table.concat(option._aliases, ",") .. '}"')
      else
         table.insert(line, '"')
         if option._maxcount > 1 then
            table.insert(line, "*")
         end
         table.insert(line, option._name)
      end
      if option._description then
         local description = get_short_description(option):gsub('["%]:`$]', "\\%0")
         table.insert(line, "[" .. description .. "]")
      end
      if option._maxargs == math.huge then
         table.insert(line, ":*")
      end
      if option._choices then
         table.insert(line, ": :(" .. table.concat(option._choices, " ") .. ")")
      elseif option._maxargs > 0 then
         table.insert(line, ": :_files")
      end
      table.insert(line, '"')
      table.insert(buf, (" "):rep(indent + 2) .. table.concat(line))
   end

   table.insert(buf, (" "):rep(indent) .. ")")
   table.insert(buf, (" "):rep(indent) .. "_arguments -s -S \\")
   table.insert(buf, (" "):rep(indent + 2) .. "$options \\")

   if self._is_help_command then
      table.insert(buf, (" "):rep(indent + 2) .. '": :(' .. self._parent:_get_commands() .. ')" \\')
   else
      for _, argument in ipairs(self._arguments) do
         local spec
         if argument._choices then
            spec = ": :(" .. table.concat(argument._choices, " ") .. ")"
         else
            spec = ": :_files"
         end
         if argument._maxargs == math.huge then
            table.insert(buf, (" "):rep(indent + 2) .. '"*' .. spec .. '" \\')
            break
         end
         for _ = 1, argument._maxargs do
            table.insert(buf, (" "):rep(indent + 2) .. '"' .. spec .. '" \\')
         end
      end

      if #self._commands > 0 then
         table.insert(buf, (" "):rep(indent + 2) .. '": :_' .. cmd_name .. '_cmds" \\')
         table.insert(buf, (" "):rep(indent + 2) .. '"*:: :->args" \\')
      end
   end

   table.insert(buf, (" "):rep(indent + 2) .. "&& return 0")
end

function Parser:_zsh_cmds(buf, cmd_name)
   table.insert(buf, "\n_" .. cmd_name .. "_cmds() {")
   table.insert(buf, "  local -a commands=(")

   for _, command in ipairs(self._commands) do
      local line = {}
      if #command._aliases > 1 then
         table.insert(line, "{" .. table.concat(command._aliases, ",") .. '}"')
      else
         table.insert(line, '"' .. command._name)
      end
      if command._description then
         table.insert(line, ":" .. get_short_description(command):gsub('["`$]', "\\%0"))
      end
      table.insert(buf, "    " .. table.concat(line) .. '"')
   end

   table.insert(buf, '  )\n  _describe "command" commands\n}')
end

function Parser:_zsh_complete_help(buf, cmds_buf, cmd_name, indent)
   if #self._commands == 0 then
      return
   end

   self:_zsh_cmds(cmds_buf, cmd_name)
   table.insert(buf, "\n" .. (" "):rep(indent) .. "case $words[1] in")

   for _, command in ipairs(self._commands) do
      local name = cmd_name .. "_" .. command._name
      table.insert(buf, (" "):rep(indent + 2) .. table.concat(command._aliases, "|") .. ")")
      command:_zsh_arguments(buf, name, indent + 4)
      command:_zsh_complete_help(buf, cmds_buf, name, indent + 4)
      table.insert(buf, (" "):rep(indent + 4) .. ";;\n")
   end

   table.insert(buf, (" "):rep(indent) .. "esac")
end

function Parser:get_zsh_complete()
   self._basename = base_name(self._name)
   assert(self:_is_shell_safe())
   local buf = {("#compdef %s\n"):format(self._basename)}
   local cmds_buf = {}
   table.insert(buf, "_" .. self._basename .. "() {")
   if #self._commands > 0 then
      table.insert(buf, "  local context state state_descr line")
      table.insert(buf, "  typeset -A opt_args\n")
   end
   self:_zsh_arguments(buf, self._basename, 2)
   self:_zsh_complete_help(buf, cmds_buf, self._basename, 2)
   table.insert(buf, "\n  return 1")
   table.insert(buf, "}")

   local result = table.concat(buf, "\n")
   if #cmds_buf > 0 then
      result = result .. "\n" .. table.concat(cmds_buf, "\n")
   end
   return result .. "\n\n_" .. self._basename .. "\n"
end

local function fish_escape(string)
   return string:gsub("[\\']", "\\%0")
end

function Parser:_fish_get_cmd(buf, indent)
   if #self._commands == 0 then
      return
   end

   table.insert(buf, (" "):rep(indent) .. "set -e cmdline[1]")
   table.insert(buf, (" "):rep(indent) .. "for arg in $cmdline")
   table.insert(buf, (" "):rep(indent + 4) .. "switch $arg")

   for _, command in ipairs(self._commands) do
      table.insert(buf, (" "):rep(indent + 8) .. "case " .. table.concat(command._aliases, " "))
      table.insert(buf, (" "):rep(indent + 12) .. "set cmd $cmd " .. command._name)
      command:_fish_get_cmd(buf, indent + 12)
      table.insert(buf, (" "):rep(indent + 12) .. "break")
   end

   table.insert(buf, (" "):rep(indent + 4) .. "end")
   table.insert(buf, (" "):rep(indent) .. "end")
end

function Parser:_fish_complete_help(buf, basename)
   local prefix = "complete -c " .. basename
   table.insert(buf, "")

   for _, command in ipairs(self._commands) do
      local aliases = table.concat(command._aliases, " ")
      local line
      if self._parent then
         line = ("%s -n '__fish_%s_using_command %s' -xa '%s'")
            :format(prefix, basename, self:_get_fullname(true), aliases)
      else
         line = ("%s -n '__fish_%s_using_command' -xa '%s'"):format(prefix, basename, aliases)
      end
      if command._description then
         line = ("%s -d '%s'"):format(line, fish_escape(get_short_description(command)))
      end
      table.insert(buf, line)
   end

   if self._is_help_command then
      local line = ("%s -n '__fish_%s_using_command %s' -xa '%s'")
         :format(prefix, basename, self:_get_fullname(true), self._parent:_get_commands())
      table.insert(buf, line)
   end

   for _, option in ipairs(self._options) do
      local parts = {prefix}

      if self._parent then
         table.insert(parts, "-n '__fish_" .. basename .. "_seen_command " .. self:_get_fullname(true) .. "'")
      end

      for _, alias in ipairs(option._aliases) do
         if alias:match("^%-.$") then
            table.insert(parts, "-s " .. alias:sub(2))
         elseif alias:match("^%-%-.+") then
            table.insert(parts, "-l " .. alias:sub(3))
         end
      end

      if option._choices then
         table.insert(parts, "-xa '" .. table.concat(option._choices, " ") .. "'")
      elseif option._minargs > 0 then
         table.insert(parts, "-r")
      end

      if option._description then
         table.insert(parts, "-d '" .. fish_escape(get_short_description(option)) .. "'")
      end

      table.insert(buf, table.concat(parts, " "))
   end

   for _, command in ipairs(self._commands) do
      command:_fish_complete_help(buf, basename)
   end
end

function Parser:get_fish_complete()
   self._basename = base_name(self._name)
   assert(self:_is_shell_safe())
   local buf = {}

   if #self._commands > 0 then
      table.insert(buf, ([[
function __fish_%s_print_command
    set -l cmdline (commandline -poc)
    set -l cmd]]):format(self._basename))
      self:_fish_get_cmd(buf, 4)
      table.insert(buf, ([[
    echo "$cmd"
end

function __fish_%s_using_command
    test (__fish_%s_print_command) = "$argv"
    and return 0
    or return 1
end

function __fish_%s_seen_command
    string match -q "$argv*" (__fish_%s_print_command)
    and return 0
    or return 1
end]]):format(self._basename, self._basename, self._basename, self._basename))
   end

   self:_fish_complete_help(buf, self._basename)
   return table.concat(buf, "\n") .. "\n"
end

local function get_tip(context, wrong_name)
   local context_pool = {}
   local possible_name
   local possible_names = {}

   for name in pairs(context) do
      if type(name) == "string" then
         for i = 1, #name do
            possible_name = name:sub(1, i - 1) .. name:sub(i + 1)

            if not context_pool[possible_name] then
               context_pool[possible_name] = {}
            end

            table.insert(context_pool[possible_name], name)
         end
      end
   end

   for i = 1, #wrong_name + 1 do
      possible_name = wrong_name:sub(1, i - 1) .. wrong_name:sub(i + 1)

      if context[possible_name] then
         possible_names[possible_name] = true
      elseif context_pool[possible_name] then
         for _, name in ipairs(context_pool[possible_name]) do
            possible_names[name] = true
         end
      end
   end

   local first = next(possible_names)

   if first then
      if next(possible_names, first) then
         local possible_names_arr = {}

         for name in pairs(possible_names) do
            table.insert(possible_names_arr, "'" .. name .. "'")
         end

         table.sort(possible_names_arr)
         return "\nDid you mean one of these: " .. table.concat(possible_names_arr, " ") .. "?"
      else
         return "\nDid you mean '" .. first .. "'?"
      end
   else
      return ""
   end
end

local ElementState = class({
   invocations = 0
})

function ElementState:__call(state, element)
   self.state = state
   self.result = state.result
   self.element = element
   self.target = element._target or element:_get_default_target()
   self.action, self.result[self.target] = element:_get_action()
   return self
end

function ElementState:error(fmt, ...)
   self.state:error(fmt, ...)
end

function ElementState:convert(argument, index)
   local converter = self.element._convert

   if converter then
      local ok, err

      if type(converter) == "function" then
         ok, err = converter(argument)
      elseif type(converter[index]) == "function" then
         ok, err = converter[index](argument)
      else
         ok = converter[argument]
      end

      if ok == nil then
         self:error(err and "%s" or "malformed argument '%s'", err or argument)
      end

      argument = ok
   end

   return argument
end

function ElementState:default(mode)
   return self.element._defmode:find(mode) and self.element._default
end

local function bound(noun, min, max, is_max)
   local res = ""

   if min ~= max then
      res = "at " .. (is_max and "most" or "least") .. " "
   end

   local number = is_max and max or min
   return res .. tostring(number) .. " " .. noun ..  (number == 1 and "" or "s")
end

function ElementState:set_name(alias)
   self.name = ("%s '%s'"):format(alias and "option" or "argument", alias or self.element._name)
end

function ElementState:invoke()
   self.open = true
   self.overwrite = false

   if self.invocations >= self.element._maxcount then
      if self.element._overwrite then
         self.overwrite = true
      else
         local num_times_repr = bound("time", self.element._mincount, self.element._maxcount, true)
         self:error("%s must be used %s", self.name, num_times_repr)
      end
   else
      self.invocations = self.invocations + 1
   end

   self.args = {}

   if self.element._maxargs <= 0 then
      self:close()
   end

   return self.open
end

function ElementState:check_choices(argument)
   if self.element._choices then
      for _, choice in ipairs(self.element._choices) do
         if argument == choice then
            return
         end
      end
      local choices_list = "'" .. table.concat(self.element._choices, "', '") .. "'"
      local is_option = getmetatable(self.element) == Option
      self:error("%s%s must be one of %s", is_option and "argument for " or "", self.name, choices_list)
   end
end

function ElementState:pass(argument)
   self:check_choices(argument)
   argument = self:convert(argument, #self.args + 1)
   table.insert(self.args, argument)

   if #self.args >= self.element._maxargs then
      self:close()
   end

   return self.open
end

function ElementState:complete_invocation()
   while #self.args < self.element._minargs do
      self:pass(self.element._default)
   end
end

function ElementState:close()
   if self.open then
      self.open = false

      if #self.args < self.element._minargs then
         if self:default("a") then
            self:complete_invocation()
         else
            if #self.args == 0 then
               if getmetatable(self.element) == Argument then
                  self:error("missing %s", self.name)
               elseif self.element._maxargs == 1 then
                  self:error("%s requires an argument", self.name)
               end
            end

            self:error("%s requires %s", self.name, bound("argument", self.element._minargs, self.element._maxargs))
         end
      end

      local args

      if self.element._maxargs == 0 then
         args = self.args[1]
      elseif self.element._maxargs == 1 then
         if self.element._minargs == 0 and self.element._mincount ~= self.element._maxcount then
            args = self.args
         else
            args = self.args[1]
         end
      else
         args = self.args
      end

      self.action(self.result, self.target, args, self.overwrite)
   end
end

local ParseState = class({
   result = {},
   options = {},
   arguments = {},
   argument_i = 1,
   element_to_mutexes = {},
   mutex_to_element_state = {},
   command_actions = {}
})

function ParseState:__call(parser, error_handler)
   self.parser = parser
   self.error_handler = error_handler
   self.charset = parser:_update_charset()
   self:switch(parser)
   return self
end

function ParseState:error(fmt, ...)
   self.error_handler(self.parser, fmt:format(...))
end

function ParseState:switch(parser)
   self.parser = parser

   if parser._action then
      table.insert(self.command_actions, {action = parser._action, name = parser._name})
   end

   for _, option in ipairs(parser._options) do
      option = ElementState(self, option)
      table.insert(self.options, option)

      for _, alias in ipairs(option.element._aliases) do
         self.options[alias] = option
      end
   end

   for _, mutex in ipairs(parser._mutexes) do
      for _, element in ipairs(mutex) do
         if not self.element_to_mutexes[element] then
            self.element_to_mutexes[element] = {}
         end

         table.insert(self.element_to_mutexes[element], mutex)
      end
   end

   for _, argument in ipairs(parser._arguments) do
      argument = ElementState(self, argument)
      table.insert(self.arguments, argument)
      argument:set_name()
      argument:invoke()
   end

   self.handle_options = parser._handle_options
   self.argument = self.arguments[self.argument_i]
   self.commands = parser._commands

   for _, command in ipairs(self.commands) do
      for _, alias in ipairs(command._aliases) do
         self.commands[alias] = command
      end
   end
end

function ParseState:get_option(name)
   local option = self.options[name]

   if not option then
      self:error("unknown option '%s'%s", name, get_tip(self.options, name))
   else
      return option
   end
end

function ParseState:get_command(name)
   local command = self.commands[name]

   if not command then
      if #self.commands > 0 then
         self:error("unknown command '%s'%s", name, get_tip(self.commands, name))
      else
         self:error("too many arguments")
      end
   else
      return command
   end
end

function ParseState:check_mutexes(element_state)
   if self.element_to_mutexes[element_state.element] then
      for _, mutex in ipairs(self.element_to_mutexes[element_state.element]) do
         local used_element_state = self.mutex_to_element_state[mutex]

         if used_element_state and used_element_state ~= element_state then
            self:error("%s can not be used together with %s", element_state.name, used_element_state.name)
         else
            self.mutex_to_element_state[mutex] = element_state
         end
      end
   end
end

function ParseState:invoke(option, name)
   self:close()
   option:set_name(name)
   self:check_mutexes(option, name)

   if option:invoke() then
      self.option = option
   end
end

function ParseState:pass(arg)
   if self.option then
      if not self.option:pass(arg) then
         self.option = nil
      end
   elseif self.argument then
      self:check_mutexes(self.argument)

      if not self.argument:pass(arg) then
         self.argument_i = self.argument_i + 1
         self.argument = self.arguments[self.argument_i]
      end
   else
      local command = self:get_command(arg)
      self.result[command._target or command._name] = true

      if self.parser._command_target then
         self.result[self.parser._command_target] = command._name
      end

      self:switch(command)
   end
end

function ParseState:close()
   if self.option then
      self.option:close()
      self.option = nil
   end
end

function ParseState:finalize()
   self:close()

   for i = self.argument_i, #self.arguments do
      local argument = self.arguments[i]
      if #argument.args == 0 and argument:default("u") then
         argument:complete_invocation()
      else
         argument:close()
      end
   end

   if self.parser._require_command and #self.commands > 0 then
      self:error("a command is required")
   end

   for _, option in ipairs(self.options) do
      option.name = option.name or ("option '%s'"):format(option.element._name)

      if option.invocations == 0 then
         if option:default("u") then
            option:invoke()
            option:complete_invocation()
            option:close()
         end
      end

      local mincount = option.element._mincount

      if option.invocations < mincount then
         if option:default("a") then
            while option.invocations < mincount do
               option:invoke()
               option:close()
            end
         elseif option.invocations == 0 then
            self:error("missing %s", option.name)
         else
            self:error("%s must be used %s", option.name, bound("time", mincount, option.element._maxcount))
         end
      end
   end

   for i = #self.command_actions, 1, -1 do
      self.command_actions[i].action(self.result, self.command_actions[i].name)
   end
end

function ParseState:parse(args)
   for _, arg in ipairs(args) do
      local plain = true

      if self.handle_options then
         local first = arg:sub(1, 1)

         if self.charset[first] then
            if #arg > 1 then
               plain = false

               if arg:sub(2, 2) == first then
                  if #arg == 2 then
                     if self.options[arg] then
                        local option = self:get_option(arg)
                        self:invoke(option, arg)
                     else
                        self:close()
                     end

                     self.handle_options = false
                  else
                     local equals = arg:find "="
                     if equals then
                        local name = arg:sub(1, equals - 1)
                        local option = self:get_option(name)

                        if option.element._maxargs <= 0 then
                           self:error("option '%s' does not take arguments", name)
                        end

                        self:invoke(option, name)
                        self:pass(arg:sub(equals + 1))
                     else
                        local option = self:get_option(arg)
                        self:invoke(option, arg)
                     end
                  end
               else
                  for i = 2, #arg do
                     local name = first .. arg:sub(i, i)
                     local option = self:get_option(name)
                     self:invoke(option, name)

                     if i ~= #arg and option.element._maxargs > 0 then
                        self:pass(arg:sub(i + 1))
                        break
                     end
                  end
               end
            end
         end
      end

      if plain then
         self:pass(arg)
      end
   end

   self:finalize()
   return self.result
end

function Parser:error(msg)
   io.stderr:write(("%s\n\nError: %s\n"):format(self:get_usage(), msg))
   os.exit(1)
end

-- Compatibility with strict.lua and other checkers:
local default_cmdline = rawget(_G, "arg") or {}

function Parser:_parse(args, error_handler)
   return ParseState(self, error_handler):parse(args or default_cmdline)
end

function Parser:parse(args)
   return self:_parse(args, self.error)
end

local function xpcall_error_handler(err)
   if not debug then
      return tostring(err)
   end
   return tostring(err) .. "\noriginal " .. debug.traceback("", 2):sub(2)
end

function Parser:pparse(args)
   local parse_error

   local ok, result = xpcall(function()
      return self:_parse(args, function(_, err)
         parse_error = err
         error(err, 0)
      end)
   end, xpcall_error_handler)

   if ok then
      return true, result
   elseif not parse_error then
      error(result, 0)
   else
      return false, parse_error
   end
end

local argparse = {}

argparse.version = "0.7.0"

setmetatable(argparse, {__call = function(_, ...)
   return Parser(default_cmdline[0]):add_help(true)(...)
end})

return argparse
