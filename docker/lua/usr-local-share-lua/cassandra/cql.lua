--[[
Implement Cassandra's native protocol v2/v3/v4
See:
  - v2: https://github.com/apache/cassandra/blob/cassandra-2.2.7/doc/native_protocol_v2.spec
  - v3: https://github.com/apache/cassandra/blob/trunk/doc/native_protocol_v3.spec
  - v4: https://github.com/apache/cassandra/blob/trunk/doc/native_protocol_v4.spec

Notes:
  - does not implement REGISTER messages
  - does not implement SUPPORTED results parsing and OPTIONS messages
  - does not implement EVENTS parsing
  - does not implement compression
  - does not set stream ids for frames
  - does not implement no_metadata query flag and ROWS results flag
  - does not implement decimal data type
  - does not implement parsing of specific error codes
    (unavailable/write_timeout/read_timeout/already_exists/etc...)
  - v4: does not implement date/time/smallint/tinyint data types
  - v4: does not implement custom payloads for custom QueryHandler
--]]

local bit = require 'bit'

local setmetatable = setmetatable
local tonumber = tonumber
local ipairs = ipairs
local pairs = pairs
local type = type
local insert = table.insert
local concat = table.concat
local ldexp = math.ldexp
local frexp = math.frexp
local floor = math.floor
local fmod = math.fmod
local huge = math.huge
local pow = math.pow
local gmatch = string.gmatch
local lower = string.lower
local char = string.char
local byte = string.byte
local gsub = string.gsub
local rep = string.rep
local sub = string.sub
local fmt = string.format
local band = bit.band
local bor = bit.bor

local Buffer = {}
local requests = {}
local frame_reader = {}

local cql_t_unset = {}
local cql_t_null = {}
local cql_types = {
  custom    = 0x00,
  ascii     = 0x01,
  bigint    = 0x02,
  blob      = 0x03,
  boolean   = 0x04,
  counter   = 0x05,
  decimal   = 0x06,
  double    = 0x07,
  float     = 0x08,
  int       = 0x09,
  text      = 0x0A,
  timestamp = 0x0B,
  uuid      = 0x0C,
  varchar   = 0x0D,
  varint    = 0x0E,
  timeuuid  = 0x0F,
  inet      = 0x10,
  list      = 0x20,
  map       = 0x21,
  set       = 0x22,
  udt       = 0x30,
  tuple     = 0x31,
}

local consistencies = {
  any               = 0x0000,
  one               = 0x0001,
  two               = 0x0002,
  three             = 0x0003,
  quorum            = 0x0004,
  all               = 0x0005,
  local_quorum      = 0x0006,
  each_quorum       = 0x0007,
  serial            = 0x0008,
  local_serial      = 0x0009,
  local_one         = 0x000a,
}

local ERRORS            = {
  SERVER                = 0x0000,
  PROTOCOL              = 0x000A,
  BAD_CREDENTIALS       = 0x0100,
  UNAVAILABLE_EXCEPTION = 0x1000,
  OVERLOADED            = 0x1001,
  IS_BOOTSTRAPPING      = 0x1002,
  TRUNCATE_ERROR        = 0x1003,
  WRITE_TIMEOUT         = 0x1100,
  READ_TIMEOUT          = 0x1200,
  READ_FAILURE          = 0x1300,
  FUNCTION_FAILURE      = 0x1400,
  WRITE_FAILURE         = 0x1500,
  SYNTAX_ERROR          = 0x2000,
  UNAUTHORIZED          = 0x2100,
  INVALID               = 0x2200,
  CONFIG_ERROR          = 0x2300,
  ALREADY_EXISTS        = 0x2400,
  UNPREPARED            = 0x2500,
}

local ERROR_TRANSLATIONS         = {
  [ERRORS.SERVER]                = 'Server error',
  [ERRORS.PROTOCOL]              = 'Protocol error',
  [ERRORS.BAD_CREDENTIALS]       = 'Bad credentials',
  [ERRORS.UNAVAILABLE_EXCEPTION] = 'Unavailable exception',
  [ERRORS.OVERLOADED]            = 'Overloaded',
  [ERRORS.IS_BOOTSTRAPPING]      = 'Is bootstrapping',
  [ERRORS.TRUNCATE_ERROR]        = 'Truncate error',
  [ERRORS.WRITE_TIMEOUT]         = 'Write timeout',
  [ERRORS.READ_TIMEOUT]          = 'Read timeout',
  [ERRORS.READ_FAILURE]          = 'Read failure',
  [ERRORS.FUNCTION_FAILURE]      = 'Function failure',
  [ERRORS.WRITE_FAILURE]         = 'Write failure',
  [ERRORS.SYNTAX_ERROR]          = 'Syntax error',
  [ERRORS.UNAUTHORIZED]          = 'Unauthorized',
  [ERRORS.INVALID]               = 'Invalid',
  [ERRORS.CONFIG_ERROR]          = 'Config error',
  [ERRORS.ALREADY_EXISTS]        = 'Already exists',
  [ERRORS.UNPREPARED]            = 'Unprepared',
}

local OP_CODES   = {
  ERROR          = 0x00,
  STARTUP        = 0x01,
  READY          = 0x02,
  AUTHENTICATE   = 0x03,
  OPTIONS        = 0x05,
  SUPPORTED      = 0x06,
  QUERY          = 0x07,
  RESULT         = 0x08,
  PREPARE        = 0x09,
  EXECUTE        = 0x0A,
  REGISTER       = 0x0B,
  EVENT          = 0x0C,
  BATCH          = 0x0D,
  AUTH_CHALLENGE = 0x0E,
  AUTH_RESPONSE  = 0x0F,
  AUTH_SUCCESS   = 0x10,
}

local FRAME_FLAGS  = {
  --COMPRESSION    = 0x01,
  TRACING          = 0x02,
  CUSTOM_PAYLOAD   = 0x04,
  WARNING          = 0x08,
}

local function is_array(t)
  if type(t) ~= "table" then return false end
  local i = 0
  for _ in pairs(t) do
    i = i + 1
    if t[i] == nil then return false end
  end
  return true
end

do

  -- Buffer
  -- @section buffer

  Buffer.__index = Buffer

  function Buffer.new(version, str)
    str = str or ''
    local buf = {
      version = version,
      str = str,
      len = #str,
      pos = 1,
    }

    return setmetatable(buf, Buffer)
  end

  function Buffer:write(bytes)
    self.str = self.str..bytes
    self.len = self.len + #bytes
    self.pos = self.len
  end

  function Buffer:read(n)
    n = n or self.len
    if n < 0 then return end

    local last_index = n ~= nil and self.pos + n - 1 or -1
    local bytes = sub(self.str, self.pos, last_index)
    self.pos = self.pos + #bytes
    return bytes
  end

  function Buffer:reset()
    self.pos = 1
  end

  function Buffer:get()
    return self.str
  end

  -- Utils
  -- @section utils

  local function big_endian_representation(num, bytes)
    if num < 0 then
      -- 2's complement
      num = pow(0x100, bytes) + num
    end
    local t = {}
    while num > 0 do
      local rest = fmod(num, 0x100)
      insert(t, 1, char(rest))
      num = (num-rest) / 0x100
    end
    local padding = rep('\0', bytes - #t)
    return padding..concat(t)
  end

  local function string_to_number(str, signed)
    local number = 0
    local exponent = 1
    for i = #str, 1, -1 do
      number = number + byte(str, i) * exponent
      exponent = exponent * 256
    end
    if signed and number > exponent / 2 then
      -- 2's complement
      number = number - exponent
    end
    return number
  end

  -- Raw types
  -- @section raw_types

  local function marsh_byte(val)
    return char(val)
  end

  local function unmarsh_byte(buffer)
    return byte(buffer:read(1))
  end

  local function marsh_int(val)
    return big_endian_representation(val, 4)
  end

  local function marsh_unset()
    return marsh_int(-2)
  end

  local function marsh_null()
    return marsh_int(-1)
  end

  local function unmarsh_int(buffer)
    return string_to_number(buffer:read(4), true)
  end

  local function marsh_long(val)
    return big_endian_representation(val, 8)
  end

  local function unmarsh_long(buffer)
    return string_to_number(buffer:read(8), true)
  end

  local function marsh_short(val)
    return big_endian_representation(val, 2)
  end

  local function unmarsh_short(buffer)
    return string_to_number(buffer:read(2), true)
  end

  local function marsh_string(val)
    return marsh_short(#val)..val
  end

  local function unmarsh_string(buffer)
    return buffer:read(buffer:read_short())
  end

  local function marsh_long_string(val)
    return marsh_int(#val)..val
  end

  local function unmarsh_long_string(buffer)
    return buffer:read(buffer:read_int())
  end

  local function marsh_bytes(val)
    return marsh_int(#val)..val
  end

  local function unmarsh_bytes(buffer)
    local n_bytes = buffer:read_int()
    return buffer:read(n_bytes)
  end

  local function marsh_short_bytes(val)
    return marsh_short(#val)..val
  end

  local function unmarsh_short_bytes(buffer)
     return buffer:read(buffer:read_short())
  end

  local function marsh_uuid(val)
    local repr = {}
    local str = gsub(val, '-', '')
    for i = 1, #str, 2 do
      local byte_str = sub(str, i, i + 1)
      repr[#repr+1] = marsh_byte(tonumber(byte_str, 16))
    end
    return concat(repr)
  end

  local function unmarsh_uuid(buffer)
    local uuid = {}
    for i = 1, 16 do
      uuid[i] = fmt('%02x', buffer:read_byte())
    end
    insert(uuid, 5, '-')
    insert(uuid, 8, '-')
    insert(uuid, 11, '-')
    insert(uuid, 14, '-')
    return concat(uuid)
  end

  local function marsh_inet(val)
    local t, hexadectets = {}, {}
    local ip = gsub(lower(val), '::', ':0000:')

    if val:match ':' then
      -- ipv6
      for hdt in gmatch(ip, '[%x]+') do
        -- fill up hexadectets with 0 so all are 4 digits long
        hexadectets[#hexadectets + 1] = rep('0', 4 - #hdt)..hdt
      end
      for i, hdt in ipairs(hexadectets) do
        while hdt == '0000' and #hexadectets < 8 do
          insert(hexadectets, i + 1, '0000')
        end
        for j = 1, 4, 2 do
          t[#t+1] = marsh_byte(tonumber(sub(hdt, j, j + 1), 16))
        end
      end
    else
      -- ipv4
      for d in gmatch(val, '(%d+)') do
        t[#t+1] = marsh_byte(d)
      end
    end

    return concat(t)
  end

  local function unmarsh_inet(buffer)
    local bytes = buffer:get()
    local buf = {}

    if #bytes == 16 then
      -- ipv6
      for i = 1, #bytes, 2 do
        buf[#buf+1] = fmt('%02x', byte(bytes, i))..fmt('%02x', byte(bytes, i+1))
      end
      return concat(buf, ':')
    else
      -- ipv4
      for i = 1, #bytes do
        buf[#buf+1] = fmt('%d', byte(bytes, i))
      end
      return concat(buf, '.')
    end
  end

  local function marsh_string_list(val)
    local t = {}
    local n = 0
    for k, v in pairs(val) do
      n = n + 1
      t[#t+1] = marsh_string(v)
    end
    insert(t, 1, marsh_short(n))
    return concat(t)
  end

  local function unmarsh_string_list(buffer)
    local list = {}
    local n = buffer:read_short()
    for _ = 1, n do
      list[#list+1] = buffer:read_string()
    end
    return list
  end

  local function marsh_string_map(val)
    local t = {}
    local n = 0
    for k, v in pairs(val) do
      n = n + 1
      t[#t+1] = marsh_string(k)
      t[#t+1] = marsh_string(v)
    end
    insert(t, 1, marsh_short(n))
    return concat(t)
  end

  local function unmarsh_string_map(buffer)
    local map = {}
    local n = buffer:read_short()
    for _ = 1, n do
      local key = buffer:read_string()
      local value = buffer:read_string()
      map[key] = value
    end
    return map
  end

  local function marsh_string_multimap(val)
    local t = {}
    local n = 0
    for k, v in pairs(val) do
      n = n + 1
      t[#t+1] = marsh_string(k)
      t[#t+1] = marsh_string_list(v)
    end
    insert(t, 1, marsh_short(n))
    return concat(t)
  end

  local function unmarsh_string_multimap(buffer)
    local multimap = {}
    local n = buffer:read_short()
    for _ = 1, n do
      local key = buffer:read_string()
      multimap[key] = buffer:read_string_list()
    end
    return multimap
  end

  local function unmarsh_udt_type(buffer)
    local udt_ks_name = buffer:read_string()
    local udt_name = buffer:read_string()
    local n = buffer:read_short()

    local fields = {}
    for _ = 1, n do
      fields[#fields+1] = {
        name = buffer:read_string(),
        type = buffer:read_options()
      }
    end

    return {
      udt_keyspace = udt_ks_name,
      udt_name = udt_name,
      fields = fields
    }
  end

  local function unmarsh_tuple_type(buffer)
    local n = buffer:read_short()

    local fields = {}
    for _ = 1, n do
      fields[#fields+1] = buffer:read_options()
    end

    return {fields = fields}
  end

  local function unmarsh_bytes_map(buffer)
    local n = buffer:read_short()

    local fields = {}
    for _ = 1, n do
      local key = buffer:read_string()
      local value = buffer:read_bytes()
      fields[key] = value
    end

    return fields
  end

  do
    local marshallers = {
      byte            = {marsh_byte, unmarsh_byte},
      int             = {marsh_int, unmarsh_int},
      long            = {marsh_long, unmarsh_long},
      short           = {marsh_short, unmarsh_short},
      string          = {marsh_string, unmarsh_string},
      long_string     = {marsh_long_string, unmarsh_long_string},
      bytes           = {marsh_bytes, unmarsh_bytes},
      short_bytes     = {marsh_short_bytes, unmarsh_short_bytes},
      uuid            = {marsh_uuid, unmarsh_uuid},
      inet            = {marsh_inet, unmarsh_inet},
      string_map      = {marsh_string_map, unmarsh_string_map},
      string_list     = {marsh_string_list, unmarsh_string_list},
      string_multimap = {marsh_string_multimap, unmarsh_string_multimap},
      udt_type        = {nil, unmarsh_udt_type},
      tuple_type      = {nil, unmarsh_tuple_type},
      bytes_map       = {nil, unmarsh_bytes_map}
    }

    for name, t in pairs(marshallers) do
      local marshaller, unmarshaller = t[1], t[2]
      if marshaller then -- skip udt/tuple
        Buffer['write_'..name] = function(self, ...)
          self:write(marshaller(...))
        end
      end
      Buffer['read_'..name] = function(self)
        return unmarshaller(self)
      end
    end
  end

  -- CQL types
  -- @section cql_types

  local function marsh_raw(val)
    return val
  end

  local function unmarsh_raw(buffer)
    return buffer:read()
  end

  local function marsh_bigint(val)
    local first_byte = val >= 0 and 0 or 0xFF
    return char(first_byte, -- only 53 bits from double
               floor(val / 0x1000000000000) % 0x100,
               floor(val / 0x10000000000) % 0x100,
               floor(val / 0x100000000) % 0x100,
               floor(val / 0x1000000) % 0x100,
               floor(val / 0x10000) % 0x100,
               floor(val / 0x100) % 0x100,
               val % 0x100)
  end

  local function unmarsh_bigint(buffer)
    local bytes = buffer:read(8)
    local b1, b2, b3, b4, b5, b6, b7, b8 = byte(bytes, 1, 8)
    if b1 < 0x80 then
      return ((((((b1 * 0x100 + b2) * 0x100 + b3) * 0x100 + b4)
               * 0x100 + b5) * 0x100 + b6) * 0x100 + b7) * 0x100 + b8
    else
      return ((((((((b1 - 0xFF) * 0x100 + (b2 - 0xFF)) * 0x100 + (b3 - 0xFF))
               * 0x100 + (b4 - 0xFF)) * 0x100 + (b5 - 0xFF)) * 0x100 + (b6 - 0xFF))
               * 0x100 + (b7 - 0xFF)) * 0x100 + (b8 - 0xFF)) - 1
    end
  end

  local function marsh_boolean(val)
    return marsh_byte(val and 1 or 0)
  end

  local function unmarsh_boolean(buffer)
    return buffer:read_byte() == 1
  end

  local function marsh_double(val)
    local sign = 0
    if val < 0.0 then
      sign = 0x80
      val = -val
    end
    local mantissa, exponent = frexp(val)
    if mantissa ~= mantissa then
      return char(0xFF, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00) -- nan
    elseif mantissa == huge then
      if sign == 0 then
        return char(0x7F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00) -- +inf
      else
        return char(0xFF, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00) -- -inf
      end
    elseif mantissa == 0.0 and exponent == 0 then
      return char(sign, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00) -- zero
    end

    exponent = exponent + 0x3FE
    mantissa = (mantissa * 2.0 - 1.0) * ldexp(0.5, 53)
    return char(sign + floor(exponent / 0x10),
               (exponent % 0x10) * 0x10 + floor(mantissa / 0x1000000000000),
               floor(mantissa / 0x10000000000) % 0x100,
               floor(mantissa / 0x100000000) % 0x100,
               floor(mantissa / 0x1000000) % 0x100,
               floor(mantissa / 0x10000) % 0x100,
               floor(mantissa / 0x100) % 0x100,
               mantissa % 0x100)
  end

  local function unmarsh_double(buffer)
    local bytes = buffer:read(8)
    local b1, b2, b3, b4, b5, b6, b7, b8 = byte(bytes, 1, 8)
    local sign = b1 > 0x7F
    local exponent = (b1 % 0x80) * 0x10 + floor(b2 / 0x10)
    local mantissa = ((((((b2 % 0x10) * 0x100 + b3) * 0x100 + b4) * 0x100 + b5)
                        * 0x100 + b6) * 0x100 + b7) * 0x100 + b8
    if sign then
      sign = -1
    else
      sign = 1
    end

    if mantissa == 0 and exponent == 0 then
      return sign * 0.0
    elseif exponent == 0x7FF then
      if mantissa == 0 then
        return sign * huge
      else
        return 0.0/0.0
      end
    end

    return sign * ldexp(1.0 + mantissa / 0x10000000000000, exponent - 0x3FF)
  end

  local function marsh_float(val)
    if val == 0 then
      return char(0x00, 0x00, 0x00, 0x00)
    elseif val ~= val then
      return char(0xFF, 0xFF, 0xFF, 0xFF)
    end

    local sign = 0x00
    if val < 0 then
      sign = 0x80
      val = -val
    end

    local mantissa, exponent = frexp(val)
    exponent = exponent + 0x7F
    if exponent <= 0 then
      mantissa = ldexp(mantissa, exponent - 1)
      exponent = 0
    elseif exponent > 0 then
      if exponent >= 0xFF then
        return char(sign + 0x7F, 0x80, 0x00, 0x00)
      elseif exponent == 1 then
        exponent = 0
      else
        mantissa = mantissa * 2 - 1
        exponent = exponent - 1
      end
    end

    mantissa = floor(ldexp(mantissa, 23) + 0.5)
    return char(sign + floor(exponent / 2),
               (exponent % 2) * 0x80 + floor(mantissa / 0x10000),
               floor(mantissa / 0x100) % 0x100,
               mantissa % 0x100)
  end

  local function unmarsh_float(buffer)
    local bytes = buffer:read(4)
    local b1, b2, b3, b4 = byte(bytes, 1, 4)
    local exponent = (b1 % 0x80) * 0x02 + floor(b2 / 0x80)
    local mantissa = ldexp(((b2 % 0x80) * 0x100 + b3) * 0x100 + b4, -23)
    if exponent == 0xFF then
      if mantissa > 0 then
        return 0 / 0
      else
        mantissa = huge
        exponent = 0x7F
      end
    elseif exponent > 0 then
      mantissa = mantissa + 1
    else
      exponent = exponent + 1
    end
    if b1 >= 0x80 then
      mantissa = -mantissa
    end

    return ldexp(mantissa, exponent - 0x7F)
  end

  -- Nested CQL types
  -- @section nested_cql_types

  local marsh_cql_value

  -- values must be ordered as they are defined in the UDT declaration
  local function marsh_udt(val, version)
    local repr = {}
    for i = 1, #val do
      repr[#repr+1] = marsh_cql_value(val[i], version)
    end
    return concat(repr)
  end

  local function unmarsh_udt(buffer, __cql_value_type)
    local udt = {}
    local fields = __cql_value_type.fields -- see unmarsh_udt_type
    for n = 1, #fields do
      local field = fields[n]
      udt[field.name] = buffer:read_cql_value(field.type)
    end
    return udt
  end

  local marsh_tuple = marsh_udt

  local function unmarsh_tuple(buffer, __cql_value_type)
    local tuple = {}
    local fields = __cql_value_type.fields -- see unmarsh_tuple_type
    for n = 1, #fields do
      tuple[#tuple+1] = buffer:read_cql_value(fields[n])
    end
    return tuple
  end

  local function marsh_set(val, version)
    local repr
    if version < 3 then
      repr = {marsh_short(#val)}
    else
      repr = {marsh_int(#val)}
    end
    for i = 1, #val do
      repr[#repr+1] = marsh_cql_value(val[i], version)
    end
    return concat(repr)
  end

  local function unmarsh_set(buffer, __cql_type_value)
    local set, n = {}
    if buffer.version < 3 then
      n = buffer:read_short()
    else
      n = buffer:read_int()
    end
    for _ = 1, n do
      set[#set+1] = buffer:read_cql_value(__cql_type_value)
    end
    return set
  end

  local function marsh_map(val, version)
    local repr = {}
    local size = 0

    for k, v in pairs(val) do
      repr[#repr+1] = marsh_cql_value(k, version)
      repr[#repr+1] = marsh_cql_value(v, version)
      size = size + 1
    end

    if version < 3 then
      insert(repr, 1, marsh_short(size))
    else
      insert(repr, 1, marsh_int(size))
    end

    return concat(repr)
  end

  local function unmarsh_map(buffer, __cql_type_value)
    local key_t, value_t = __cql_type_value[1], __cql_type_value[2]
    local map = {}

    local n
    if buffer.version < 3 then
      n = buffer:read_short()
    else
      n = buffer:read_int()
    end

    for _ = 1, n do
      local key = buffer:read_cql_value(key_t)
      map[key] = buffer:read_cql_value(value_t)
    end

    return map
  end

  function Buffer:read_options()
    local cql_t, cql_t_val = self:read_short()
    if cql_t == cql_types.set or cql_t == cql_types.list then
      cql_t_val = self:read_options()
    elseif cql_t == cql_types.map then
      cql_t_val = {self:read_options(), self:read_options()}
    elseif cql_t == cql_types.udt then
      cql_t_val = unmarsh_udt_type(self)
    elseif cql_t == cql_types.tuple then
      cql_t_val = unmarsh_tuple_type(self)
    end

    return {
      __cql_type = cql_t,
      __cql_type_value = cql_t_val
    }
  end

  -- CQL Marshalling
  -- @section cql_marshalling

  local cql_marshallers = {
    -- custom           = 0x00,
    [cql_types.ascii]   = marsh_raw,
    [cql_types.bigint]  = marsh_bigint,
    [cql_types.blob]    = marsh_raw,
    [cql_types.boolean] = marsh_boolean,
    [cql_types.counter] = marsh_bigint,
    -- decimal 0x06
    [cql_types.double]    = marsh_double,
    [cql_types.float]     = marsh_float,
    [cql_types.inet]      = marsh_inet,
    [cql_types.int]       = marsh_int,
    [cql_types.text]      = marsh_raw,
    [cql_types.list]      = marsh_set,
    [cql_types.map]       = marsh_map,
    [cql_types.set]       = marsh_set,
    [cql_types.uuid]      = marsh_uuid,
    [cql_types.timestamp] = marsh_bigint,
    [cql_types.varchar]   = marsh_raw,
    [cql_types.varint]    = marsh_int,
    [cql_types.timeuuid]  = marsh_uuid,
    [cql_types.udt]       = marsh_udt,
    [cql_types.tuple]     = marsh_tuple
  }

  marsh_cql_value = function(val, version)
    if val == cql_t_unset then
      return marsh_unset()
    elseif val == cql_t_null then
      return marsh_null()
    end

    local cql_t
    local typ = type(val)
    if typ == 'table' then
      -- set by cassandra.uuid() or the likes
      if val.__cql_type then
        cql_t = val.__cql_type
        val = val.val
      elseif is_array(val) then
        cql_t = cql_types.list
      else
        cql_t = cql_types.map
      end
    elseif typ == 'number' then
      if floor(val) == val then
        cql_t = cql_types.int
      else
        cql_t = cql_types.float
      end
    elseif typ == 'boolean' then
      cql_t = cql_types.boolean
    else
      cql_t = cql_types.varchar
    end

    local marshaller = cql_marshallers[cql_t]
    if not marshaller then
      error('no marshaller for CQL type '..cql_t)
    end

    return marsh_bytes(marshaller(val, version))
  end

  function Buffer:write_cql_value(val)
    self:write(marsh_cql_value(val, self.version))
  end

  -- CQL Unmarshalling
  -- @section cql_unmarshalling

  local cql_unmarshallers = {
    -- custom             = 0x00,
    [cql_types.ascii]     = unmarsh_raw,
    [cql_types.bigint]    = unmarsh_bigint,
    [cql_types.blob]      = unmarsh_raw,
    [cql_types.boolean]   = unmarsh_boolean,
    [cql_types.counter]   = unmarsh_bigint,
    -- decimal 0x06
    [cql_types.double]    = unmarsh_double,
    [cql_types.float]     = unmarsh_float,
    [cql_types.inet]      = unmarsh_inet,
    [cql_types.int]       = unmarsh_int,
    [cql_types.text]      = unmarsh_raw,
    [cql_types.list]      = unmarsh_set,
    [cql_types.map]       = unmarsh_map,
    [cql_types.set]       = unmarsh_set,
    [cql_types.uuid]      = unmarsh_uuid,
    [cql_types.timestamp] = unmarsh_bigint,
    [cql_types.varchar]   = unmarsh_raw,
    [cql_types.varint]    = unmarsh_int,
    [cql_types.timeuuid]  = unmarsh_uuid,
    [cql_types.udt]       = unmarsh_udt,
    [cql_types.tuple]     = unmarsh_tuple
  }

  -- Read a CQL value with a given CQL type
  function Buffer:read_cql_value(t)
    local unmarshaller = cql_unmarshallers[t.__cql_type]
    if not unmarshaller then
      error('no unmarshaller for CQL type '..t.__cql_type)
    end
    local bytes = self:read_bytes()
    if bytes then
      local buffer = Buffer.new(self.version, bytes)
      return unmarshaller(buffer, t.__cql_type_value)
    end
  end
end -- do CQL encoding

-- CQL Requests
-- @section cql_requests

do
  local CQL_VERSION = '3.0.0'
  local QUERY_FLAGS = {
    VALUES                 = 0x01,
    --SKIP_METADATA        = 0x02,
    PAGE_SIZE              = 0x04,
    WITH_PAGING_STATE      = 0x08,
    WITH_SERIAL_CONSISTENCY = 0x10,
    WITH_DEFAULT_TIMESTAMP = 0x20,
    WITH_NAMES_FOR_VALUES  = 0x40,
  }

  local request_mt = {}
  request_mt.__index = request_mt

  local function new_request(op_code)
    return setmetatable({
      retries = 0,
      header = Buffer.new(),
      body = Buffer.new(),
      op_code = op_code
    }, request_mt)
  end

  function request_mt:build_frame(version)
    local header, body = Buffer.new(version), Buffer.new(version)

    self:build_body(body)        -- build body (depends on protocol version)
    header:write_byte(version)

    local flags = 0
    if self.opts and self.opts.tracing then
      flags = bor(flags, FRAME_FLAGS.TRACING)
    end

    header:write_byte(flags)

    if version < 3 then
      header:write_byte(0)       -- stream_id
    else
      header:write_short(0)      -- stream_id
    end

    header:write_byte(self.op_code)
    header:write_int(body.len)

    return header:get()..body:get()
  end

  local default_opts = {consistency = consistencies.one}
  local function build_args(body, args, opts)
    opts = opts or default_opts

    local flags = 0x00
    local buf = Buffer.new(body.version)
    if args then
      flags = bor(flags, QUERY_FLAGS.VALUES)

      if body.version >= 3 and opts.named then
        flags = bor(flags, QUERY_FLAGS.WITH_NAMES_FOR_VALUES)
        local n = 0
        local args_buf = Buffer.new(body.version)
        for name, val in pairs(args) do
          n = n + 1
          args_buf:write_string(name)
          args_buf:write_cql_value(val)
        end
        buf:write_short(n)
        buf:write(args_buf:get()) -- append args
      else
        buf:write_short(#args)
        for i = 1, #args do
          buf:write_cql_value(args[i])
        end
      end
    end
    if opts.page_size then
      flags = bor(flags, QUERY_FLAGS.PAGE_SIZE)
      buf:write_int(opts.page_size)
    end
    if opts.paging_state then
      flags = bor(flags, QUERY_FLAGS.WITH_PAGING_STATE)
      buf:write_bytes(opts.paging_state)
    end

    if body.version >= 3 then
      if opts.serial_consistency then
        flags = bor(flags, QUERY_FLAGS.WITH_SERIAL_CONSISTENCY)
        buf:write_short(opts.serial_consistency)
      end
      if opts.timestamp then
        flags = bor(flags, QUERY_FLAGS.WITH_DEFAULT_TIMESTAMP)
        buf:write_long(opts.timestamp)
      end
    end

    body:write_short(opts.consistency)
    body:write_byte(flags)
    body:write(buf:get())
  end

  local req_mts = {
    startup = {
      new = function()
        return new_request(OP_CODES.STARTUP)
      end,
      build_body = function(_, body)
        body:write_string_map {
          ['CQL_VERSION'] = CQL_VERSION
        }
      end
    },
    keyspace = {
      new = function(keyspace)
        local r = new_request(OP_CODES.QUERY)
        r.keyspace = keyspace
        return r
      end,
      build_body = function(self, body)
        body:write_long_string(fmt([[USE "%s"]], self.keyspace))
        build_args(body)
      end
    },
    query = {
      new = function(query, args, opts)
        local r = new_request(OP_CODES.QUERY)
        r.query = query
        r.args = args
        r.opts = opts
        return r
      end,
      build_body = function(self, body)
        body:write_long_string(self.query)
        build_args(body, self.args, self.opts)
      end
    },
    prepare = {
      new = function(query)
        local r = new_request(OP_CODES.PREPARE)
        r.query = query
        return r
      end,
      build_body = function(self, body)
        body:write_long_string(self.query)
      end
    },
    execute_prepared = {
      new = function(query_id, args, opts, query)
        local r = new_request(OP_CODES.EXECUTE)
        r.query_id = query_id
        r.args = args
        r.opts = opts
        r.query = query -- allow to be re-prepared by cluster
        return r
      end,
      build_body = function(self, body)
        body:write_short_bytes(self.query_id)
        build_args(body, self.args, self.opts)
      end
    },
    batch = {
      new = function(queries, opts)
        local r = new_request(OP_CODES.BATCH)
        r.queries = queries
        r.opts = opts
        r.type = opts.logged and 0 or 1
        r.type = opts.counter and 2 or r.type
        return r
      end,
      build_body = function(self, body)
        local queries = self.queries
        local opts = self.opts or default_opts
        body:write_byte(self.type)
        body:write_short(#queries)
        for i = 1, #queries do
          local q = queries[i] -- {query, args, query_id}
          if opts.prepared then
            body:write_byte(1)
            body:write_short_bytes(q[3])
          else
            body:write_byte(0)
            body:write_long_string(q[1])
          end
          if q[2] then
            local args = q[2]
            --[[
            not supported because of CQL issue we reported:
            https://issues.apache.org/jira/browse/CASSANDRA-10246
            if body.version >= 3 and opts.named then
              local n = 0
              local args_buf = Buffer.new(body.version)
              for name, val in pairs(args) do
                n = n + 1
                args_buf:write_string(name)
                args_buf:write_cql_value(val)
              end
              body:write_short(n)
              body:write(args_buf:get()) -- append args
            else
            --]]
              body:write_short(#args)
              for i = 1, #args do
                body:write_cql_value(args[i])
              end
            --end
          else
            body:write_short(0)
          end
        end

        body:write_short(opts.consistency)

        if body.version >= 3 then
          local flags = 0x00
          local buf = Buffer.new(body.version)
          if opts.serial_consistency then
            flags = bor(flags, QUERY_FLAGS.WITH_SERIAL_CONSISTENCY)
            buf:write_short(opts.serial_consistency)
          end
          if opts.timestamp then
            flags = bor(flags, QUERY_FLAGS.WITH_DEFAULT_TIMESTAMP)
            buf:write_long(opts.timestamp)
          end
          --[[
          if opts.named then
            flags = bor(flags, 0x40)
          end
          --]]
          body:write_byte(flags)
          body:write(buf:get())
        end
      end
    },
    auth_response = {
      new = function(token)
        local r = new_request(OP_CODES.AUTH_RESPONSE)
        r.token = token
        return r
      end,
      build_body = function(self, body)
        body:write_bytes(self.token)
      end
    }
  }

  for k, v in pairs(req_mts) do
    requests[k] = {
      new = function(...)
        local r = v.new(...)
        r.build_body = v.build_body
        return r
      end
    }
  end
end

-- Frame reader
-- @section fame_reader

do
  local RESULT_KINDS = {
    VOID             = 0x01,
    ROWS             = 0x02,
    SET_KEYSPACE     = 0x03,
    PREPARED         = 0x04,
    SCHEMA_CHANGE    = 0x05,
  }

  local ROWS_RESULT_FLAGS = {
    GLOBAL_TABLES_SPEC    = 0x01,
    HAS_MORE_PAGES        = 0x02,
    NO_METADATA           = 0x04,
  }

  function frame_reader.version(b)
    local version = band(byte(b), 0x7F) -- string.byte
    local header_size = version < 3 and 8 or 9
    return version, header_size - 1 -- -1 because b was our first byte
  end

  function frame_reader.read_header(version, bytes)
    local buf = Buffer.new(version, bytes)
    local flags = buf:read_byte()
    local stream_id
    if version < 3 then
      stream_id = buf:read_byte()
    else
      stream_id = buf:read_short()
    end
    return {
      flags       = flags,
      version     = version,
      stream_id   = stream_id,
      op_code     = buf:read_byte(),
      body_length = buf:read_int()
    }
  end

  local function parse_v4_prepared_metadata(body)
    local partition_keys, columns = {}, {}
    local k_name, t_name

    local flags = body:read_int()
    local columns_count = body:read_int()
    local pk_count = body:read_int()
    local has_global_table_spec = band(flags, ROWS_RESULT_FLAGS.GLOBAL_TABLES_SPEC) ~= 0

    for _ = 1, pk_count do
      partition_keys[#partition_keys+1] = body:read_short()
    end

    if has_global_table_spec then
      k_name = body:read_string()
      t_name = body:read_string()
    end

    for _ = 1, columns_count do
      if not has_global_table_spec then
        k_name = body:read_string()
        t_name = body:read_string()
      end
      columns[#columns+1] = {
        name = body:read_string(),
        type = body:read_options(),
        keysapce = k_name,
        table = t_name
      }
    end

    return {
      columns        = columns,
      columns_count  = columns_count,
      partition_keys = partition_keys
    }
  end

  local function parse_metadata(body)
    local columns = {}
    local k_name, t_name, paging_state

    local flags = body:read_int()
    local columns_count = body:read_int()
    local has_more_pages = band(flags, ROWS_RESULT_FLAGS.HAS_MORE_PAGES) ~= 0
    local has_global_table_spec = band(flags, ROWS_RESULT_FLAGS.GLOBAL_TABLES_SPEC) ~= 0
    --local has_no_metadata = band(flags, ROWS_RESULT_FLAGS.NO_METADATA) ~= 0

    if has_more_pages then
      paging_state = body:read_bytes()
    end
    if has_global_table_spec then
      k_name = body:read_string()
      t_name = body:read_string()
    end

    for _ = 1, columns_count do
      if not has_global_table_spec then
        k_name = body:read_string()
        t_name = body:read_string()
      end
      columns[#columns+1] = {
        name = body:read_string(),
        type = body:read_options(),
        keysapce = k_name,
        table = t_name
      }
    end
    return {
      columns        = columns,
      columns_count  = columns_count,
      has_more_pages = has_more_pages,
      paging_state   = paging_state
    }
  end

  local results_parsers = {
    [RESULT_KINDS.VOID] = function()
      return {type = 'VOID'}
    end,
    [RESULT_KINDS.ROWS] = function(body)
      local metadata = parse_metadata(body)
      local columns = metadata.columns
      local columns_count = metadata.columns_count
      local rows_count = body:read_int()

      local rows = {
        type = 'ROWS',
        meta = {
          has_more_pages = metadata.has_more_pages,
          paging_state = metadata.paging_state
        }
      }

      for _ = 1, rows_count do
        local row = {}
        for i = 1, columns_count do
          row[columns[i].name] = body:read_cql_value(columns[i].type)
        end
        rows[#rows+1] = row
      end
      return rows
    end,
    [RESULT_KINDS.SET_KEYSPACE] = function(body)
      return {
        type = 'SET_KEYSPACE',
        keyspace = body:read_string()
      }
    end,
    [RESULT_KINDS.PREPARED] = function(body)
      local query_id = body:read_short_bytes()
      local metadata
      if body.version == 4 then
        metadata = parse_v4_prepared_metadata(body)
      else
        metadata = parse_metadata(body)
      end
      local result_metadata = parse_metadata(body)
      return {
        type     = 'PREPARED',
        meta     = metadata,
        query_id = query_id,
        result   = result_metadata
      }
    end,
    [RESULT_KINDS.SCHEMA_CHANGE] = function(body)
      local change_type = body:read_string()
      local target = body:read_string()
      local keyspace = body:read_string()
      local name, args_types
      if target == 'TABLE' or target == 'TYPE' then
        name = body:read_string()
      elseif target == 'FUNCTION' or target == 'AGGREGATE' then
        -- v4 only
        name = body:read_string()
        args_types = body:read_string_list()
      end
      return {
        type            = 'SCHEMA_CHANGE',
        name            = name,
        target          = target,
        keyspace        = keyspace,
        arguments_types = args_types,
        change_type     = change_type
      }
    end
  }

  local ready = {ready = true}

  function frame_reader.read_body(header, bytes)
    local op_code = header.op_code
    local body = Buffer.new(header.version, bytes)

    local tracing_id, warnings
    if band(header.flags, FRAME_FLAGS.TRACING) ~= 0 then
      tracing_id = body:read_uuid()
    end
    if band(header.flags, FRAME_FLAGS.CUSTOM_PAYLOAD) ~= 0 then
      body:read_bytes_map() -- discard
    end
    if band(header.flags, FRAME_FLAGS.WARNING) ~= 0 then
      warnings = body:read_string_list()
    end

    if op_code == OP_CODES.RESULT then
      local result_kind = body:read_int()
      local parser = results_parsers[result_kind]
      local res = parser(body)
      res.tracing_id = tracing_id
      res.warnings = warnings
      return res
    elseif op_code == OP_CODES.ERROR then
      local code = body:read_int()
      local message = body:read_string()
      local error_translation = ERROR_TRANSLATIONS[code]

      -- If the translation is not found, return a formatted string
      -- with the error code for convenience.
      if error_translation == nil then
        error_translation = fmt('UNSUPPORTED ERROR (code=0x%x)', code)
      end

      return nil, '['.. error_translation ..'] '..message, code
    elseif op_code == OP_CODES.READY then
      return ready
    elseif op_code == OP_CODES.AUTHENTICATE then
      local class_name = body:read_string()
      return {must_authenticate = true, class_name = class_name}
    elseif op_code == OP_CODES.AUTH_SUCCESS then
      local token = body:read_bytes()
      return {authenticated = true, token = token}
    end
  end
end

return {
  errors               = ERRORS,
  requests             = requests,
  types                = cql_types,
  t_unset              = cql_t_unset,
  t_null               = cql_t_null,
  frame_reader         = frame_reader,
  consistencies        = consistencies,
  min_protocol_version = 2,
  def_protocol_version = 4,

  -- for testing only
  is_array = is_array,
  buffer   = Buffer
}
