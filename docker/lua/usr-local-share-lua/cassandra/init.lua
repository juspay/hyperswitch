--- Cassandra single-host client module.
-- Single host module for PUC Lua, LuaJIT and OpenResty.
-- @module cassandra
-- @author thibaultcha
-- @release 1.5.2

local socket = require 'cassandra.socket'
local cql = require 'cassandra.cql'

local setmetatable = setmetatable
local requests = cql.requests
local fmt = string.format
local pairs = pairs
local find = string.find

--- CQL error codes
-- CQL error codes constant. Useful when it is desired to programatically
-- determine the type of error that occurred during a query execution.
-- @field SERVER Something unexpected happened. This indicates a server-side
-- bug.
-- @field PROTOCOL Some client message triggered a protocol violation
-- (for instance a QUERY message is sent before a STARTUP).
-- @field BAD_CREDENTIALS A CREDENTIALS request failed because Cassandra did
-- not accept the provided credentials.
-- @field UNAVAILABLE_EXCEPTION The query could not be processed with respect
-- to the given concurrency.
-- @field OVERLOADED The request cannot be processed because the coordinator
-- node is overloaded.
-- @field IS_BOOTSTRAPPING The request was a read request but the coordinator
-- node is bootstrapping.
-- @field TRUNCATE_ERROR Error during a truncation.
-- @field WRITE_TIMEOUT Timeout exception during a write request.
-- @field READ_TIMEOUT Timeout exception during a read request.
-- @field SYNTAX_ERROR The submitted query has a syntax error.
-- @field UNAUTHORIZED The logged-in user doesn't have the right to perform
-- the query.
-- @field INVALID The query is syntactically correct but invalid.
-- @field CONFIG_ERROR The query is invalid because of some configuration
-- issue.
-- @field ALREADY_EXISTS The query attempted to create a keyspace or a table
-- that is already existing.
-- @field UNPREPARED Can be thrown while a prepared statement tries to be
-- executed if the provided prepared query id is not known by this host.
-- @table cassandra.cql_errors

--- CQL consistencies.
-- @field all CQL consistency `ALL`.
--     cassandra.consistencies.all
-- @field each_quorum CQL consistency `EACH_QUORUM`.
--     cassandra.consistencies.each_quorum
-- @field quorum CQL consistency `QUORUM`.
--     cassandra.consistencies.quorum
-- @field local_quorum CQL consistency `LOCAL_QUORUM`.
--     cassandra.consistencies.local_quorum
-- @field one CQL consistency `ONE`.
--     cassandra.consistencies.one
-- @field two CQL consistency `TWO`.
--     cassandra.consistencies.two
-- @field three CQL consistency `THREE`.
--     cassandra.consistencies.three
-- @field local_one CQL consistency `LOCAL_ONE`.
--     cassandra.consistencies.local_one
-- @field any CQL consistency `ANY`.
--     cassandra.consistencies.any
-- @field serial CQL consistency `SERIAL`.
--     cassandra.consistencies.seriam
-- @field local_serial CQL consistency `LOCAL_SERIAL`.
--     cassandra.consistencies.local_serial
-- @table cassandra.consistencies

--- Authentication providers
-- @field plain_text The plain text auth provider.
--     local auth = cassandra.auth_provider.plain_text("username", "password")
-- @table cassandra.auth_providers

local _Host = {
  _VERSION = '1.5.2',
  cql_errors = cql.errors,
  consistencies = cql.consistencies,
  auth_providers = require 'cassandra.auth'
}

_Host.__index = _Host

--- New client options.
-- Options taken by `new` upon client creation.
-- @field host Address to which the client should connect.
-- (`string`, default: `"127.0.0.1"`)
-- @field port Port to which the client should connect.
-- (`number`, default: `9042`)
-- @field keyspace Keyspace the client should use. (`string`, optional)
-- @field protocol_version Binary protocol version the client should try to use
-- (`number`, default: `3`)
-- @field ssl Determines if the client should connect using SSL.
-- (`boolean`, default: `false`)
-- @field ssl_protocol The client encryption protocol version to use
-- if `ssl` is enabled (LuaSec usage only, see `lua_ssl_protocols` directive
-- for ngx_lua).
-- (`string`, default: `any`)
-- @field verify Enable server certificate validation if `ssl` is enabled.
-- (`boolean`, default: `false`)
-- @field cafile Path to the server certificate (LuaSec usage only, see
-- `lua_ssl_trusted_certificate` directive for ngx_lua).
-- (`string`, optional)
-- @field cert Path to the client SSL certificate (LuaSec usage only).
-- (`string`, optional)
-- @field key Path to the client SSL key (LuaSec usage only).
-- (`string`, optional)
-- @field auth Authentication handler, created from the
-- `cassandra.auth_providers` table. (optional)
-- @table `client_options`

--- Create a new Cassandra client.
-- Takes a table of `client_options`. Does not connect automatically.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new {
--   host = "10.0.0.1",
--   port = 9042,
--   keyspace = "my_keyspace"
-- }
--
-- @param[type=table] opts Options for the created client.
-- @treturn table `client`: A table able to connect to the given host and port.
function _Host.new(opts)
  opts = opts or {}
  local sock, err = socket.tcp()
  if err then return nil, err end

  local host = {
    sock = sock,
    host = opts.host or '127.0.0.1',
    port = opts.port or 9042,
    keyspace = opts.keyspace,
    protocol_version = opts.protocol_version or cql.def_protocol_version,
    ssl = opts.ssl,
    verify = opts.verify,
    cert = opts.cert,
    cafile = opts.cafile,
    ssl_protocol = opts.ssl_protocol,
    key = opts.key,
    auth = opts.auth
  }

  return setmetatable(host, _Host)
end

function _Host:send(request)
  if not self.sock then
    return nil, 'no socket created'
  end

  local frame = request:build_frame(self.protocol_version)
  local sent, err = self.sock:send(frame)
  if not sent then return nil, err end

  -- receive frame version byte
  local v_byte, err = self.sock:receive(1)
  if not v_byte then return nil, err end

  -- -1 because of the v_byte we just read
  local version, n_bytes = cql.frame_reader.version(v_byte)

  -- receive frame header
  local header_bytes, err = self.sock:receive(n_bytes)
  if not header_bytes then return nil, err end

  local header = cql.frame_reader.read_header(version, header_bytes)

  -- receive frame body
  local body_bytes
  if header.body_length > 0 then
    body_bytes, err = self.sock:receive(header.body_length)
    if not body_bytes then return nil, err end
  end

  -- res, err, cql_err_code
  return cql.frame_reader.read_body(header, body_bytes)
end

local function send_startup(self)
  local startup_req = requests.startup.new()
  return self:send(startup_req)
end

local function send_auth(self)
  local token = self.auth:initial_response()
  local auth_request = requests.auth_response.new(token)
  local res, err = self:send(auth_request)
  if not res then
    return nil, err
  elseif res and res.authenticated then
    return true
  end
end

local function ssl_handshake(self)
  local params = {
    key = self.key,
    cafile = self.cafile,
    protocol = self.ssl_protocol,
    cert = self.cert
  }

  return self.sock:sslhandshake(false, nil, self.verify, params)
end

--- Connect to the remote node.
-- Uses the `client_options` given at creation to connect to the configured
-- Cassandra node.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new()
-- assert(client:connect())
--
-- @treturn boolean `ok`: `true` if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
function _Host:connect()
  if not self.sock then
    return nil, 'no socket created'
  end

  local ok, err = self.sock:connect(self.host, self.port, {
    pool = fmt('%s:%d:%s', self.host, self.port, self.keyspace or '')
  })
  if not ok then return nil, err, true end

  if self.ssl then
    ok, err = ssl_handshake(self)
    if not ok then return nil, 'SSL handshake: '..err end
  end

  local reused, err = self.sock:getreusedtimes()
  if not reused then return nil, err end

  if reused < 1 then
    -- startup request on first connection
    local res, err, code = send_startup(self)
    if not res then
      if code == cql.errors.PROTOCOL and
        find(err, 'Invalid or unsupported protocol version', nil, true) then
        -- too high protocol version
        self.sock:close()
        local sock, err = socket.tcp()
        if err then return nil, err end
        self.sock = sock
        self.protocol_version = self.protocol_version - 1
        if self.protocol_version < cql.min_protocol_version then
          return nil, 'could not find a supported protocol version'
        end
        return self:connect()
      end

      -- real connection issue, host could be down?
      return nil, err, true
    elseif res.must_authenticate then
      if not self.auth then
        return nil, 'authentication required'
      end

      local ok, err = send_auth(self)
      if not ok then return nil, err end
    end

    if self.keyspace then
      local keyspace_req = requests.keyspace.new(self.keyspace)
      local res, err = self:send(keyspace_req)
      if not res then return nil, err end
    end
  end

  return true
end

--- Set the timeout value.
-- @see https://github.com/openresty/lua-nginx-module#tcpsocksettimeout
-- @param[type=number] timeout Value in milliseconds (for connect/read/write).
-- @treturn boolean `ok`: `true` if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
function _Host:settimeout(...)
  if not self.sock then
    return nil, 'no socket created'
  end
  self.sock:settimeout(...)
  return true
end

--- Put the underlying socket into the cosocket connection pool.
-- Keeps the underlying socket alive until other clients use the `connect`
-- method on the same host/port combination.
-- @see https://github.com/openresty/lua-nginx-module#tcpsocksetkeepalive
-- @param[type=number] timeout (optional) Value in milliseconds specifying the
-- maximal idle timeout.
-- @param[type=number] size (optional) Maximal number of connections allowed in
-- the pool for the current server.
-- @treturn number `success`: `1` if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
function _Host:setkeepalive(...)
  if not self.sock then
    return nil, 'no socket created'
  end
  return self.sock:setkeepalive(...)
end

--- Close the connection.
-- @see https://github.com/openresty/lua-nginx-module#tcpsockclose
-- @treturn number `success`: `1` if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
function _Host:close(...)
  if not self.sock then
    return nil, 'no socket created'
  end
  return self.sock:close(...)
end

--- Change the client's keyspace.
-- Closes the current connection and open a new one to the given
-- keyspace.
-- The connection is closed and reopen so that we use a different connection
-- pool for usage in ngx_lua.
-- @param[type=string] keyspace Name of the desired keyspace.
-- @treturn boolean `ok`: `true` if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
function _Host:change_keyspace(keyspace)
  local _, err = self:close()
  if err then return nil, err end

  local sock, err = socket.tcp()
  if err then return nil, err end

  self.sock = sock
  self.keyspace = keyspace

  return self:connect()
end

--- Query options.
-- @field consistency Consistency level for this request.
-- See `cassandra.consistencies` table.
-- (default: `cassandra.consistencies.one`)
-- @field serial_consistency Serial consistency level for this request.
-- See `cassandra.consistencies` table.
-- (default: `cassandra.consistencies.serial`)
-- @field page_size When retrieving a set of rows (`SELECT`), determines the
-- maximum maximum amount of rows per page.
-- (`number`, default: `1000`)
-- @field paging_state String token representing the paging state. Useful for
-- manual paging: if provided, the query will be executed
-- starting from the given paging state.
-- (`string`, optional)
-- @field tracing Enable query tracing. Use this option to diagnose performance
-- problems related to query execution.
-- (`boolean`, default: `false`)
-- @field prepared Determines if the argument given to `execute` is a prepared
-- query id (from `prepare`) to be executed.
-- (`boolean`, default: `false`)
-- @field logged When executing a `batch`, determines if the batch should be
-- written to the batchlog. (`boolean`, default: `true`)
-- @field counter When executing a `batch`, specify if the batch contains
-- counter updates. (`boolean`, default: `false`)
-- @field timestamp The default timestamp for the query/batch in microseconds
-- from unix epoch. If provided, will replace the server
-- side assigned timestamp as default timestamp.
-- (`number`, optional)
-- @field named Determines if arguments binded to `execute` are key/value
-- indexed instead of an array. (`boolean`, default: `false`)
-- @table query_options

local query_options = {
  consistency = cql.consistencies.one,
  serial_consistency = cql.consistencies.serial,
  page_size = 1000,
  paging_state = nil,
  tracing = false,
  -- execute with a prepared query id
  prepared = false,
  -- batch
  logged = true,
  counter = false,
  -- protocol v3+ options
  timestamp = nil,
  named = false,
}

local function get_opts(o)
  if not o then
    return query_options
  else
    local opts = {
      paging_state = o.paging_state,
      timestamp = o.timestamp
    }
    for k, v in pairs(query_options) do
      if o[k] == nil then
        opts[k] = v
      else
        opts[k] = o[k]
      end
    end
    return opts
  end
end

_Host.get_request_opts = get_opts

local function page_iterator(self, query, args, opts)
  opts = opts or {}
  local page = 0
  return function(_, p_rows)
    local meta = p_rows.meta
    if not meta.has_more_pages then return end -- end after error

    opts.paging_state = meta.paging_state

    local rows, err = self:execute(query, args, opts)
    if rows and #rows > 0 then
      page = page + 1
    elseif err then -- expose the error with one more iteration
      rows = {meta = {has_more_pages = false}}
    else -- end of iteration
      return nil
    end

    return rows, err, page
  end, nil, {meta = {has_more_pages = true}}
  -- nil: our iteration has no invariant state, our control variable is
  -- the rows themselves
end

_Host.page_iterator = page_iterator

--- Prepare a query.
-- Sends a PREPARE request for the given query. The result of this request will
-- contain a query id, which can be given to `execute` if the `prepared` option
-- is enabled.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new()
-- assert(client:connect())
--
-- local res = assert(client:prepare("SELECT * FROM users WHERE id = ?"))
-- local rows = assert(client:execute(res.query_id, {12345}, {prepared = true}))
-- print(#rows) -- 1
--
-- @param[type=string] query CQL query to prepare.
-- @treturn table `res`: Table holding the query result if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
-- @treturn number `cql_err`: If a server-side error occurred, the CQL error code.
function _Host:prepare(query)
  local prepare_request = requests.prepare.new(query)
  return self:send(prepare_request)
end

--- Execute a query.
-- Sends a request to execute the given query.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new()
-- assert(client:connect())
--
-- local rows = assert(client:execute("SELECT * FROM users WHERE name = ? AND email = ?", {
--   "john",
--   "john@gmail.com"
-- }))
-- print(#rows) -- 1
--
-- local rows, err, cql_code = client:execute("SELECT * FROM users WHERE age = ?", {
--   age = 21
-- }, {
--   named = true,
--   page_size = 5000
-- })
-- if not rows then
--   -- can compare cql_code to determine error type
--   error(err)
-- end
-- print(#rows) -- `<= 5000`
-- print(rows.meta.paging_state) -- pagination token
--
-- @param[type=string] query CQL query to execute.
-- @param[type=table] args (optional) Arguments to bind to the query.
-- @param[type=table] options (optional) Options from `query_options`
-- for this query.
-- @treturn table `res`: Table holding the query result if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
-- @treturn number `cql_err`: If a server-side error occurred, the CQL error code.
function _Host:execute(query, args, options)
  local opts = get_opts(options)
  local request = opts.prepared and
    -- query is the prepared query id
    requests.execute_prepared.new(query, args, opts)
    or
    requests.query.new(query, args, opts)

  return self:send(request)
end

--- Execute a batch.
-- Sends a request to execute the given batch.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new()
-- assert(client:connect())
--
-- local res = assert(client:batch({
--   {"INSERT INTO things(id, n) VALUES(?, 1)", {123}},
--   {"UPDATE things SET n = 2 WHERE id = ?", {123}},
--   {"UPDATE things SET n = 3 WHERE id = ?", {123}}
-- }, {
--   logged = false
-- }))
--
-- @param[type=table] queries Array of CQL queries to execute as a batch.
-- @param[type=table] options (optional) Options from `query_options`
-- for this query.
-- @treturn table `res`: Table holding the query result if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
-- @treturn number `cql_err`: If a server-side error occurred, the CQL error code.
function _Host:batch(queries, options)
  local batch_request = requests.batch.new(queries, get_opts(options))
  return self:send(batch_request)
end

--- Lua iterator for auto-pagination.
-- Perform auto-pagination for a query when used as a Lua iterator.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new()
-- assert(client:connect())
--
-- for rows, err, page in client:iterate("SELECT * FROM users") do
--   if err then
--     error(err)
--   end
--   print(page)
--   print(#rows)
-- end
--
-- @param[type=string] query CQL query to execute.
-- @param[type=table] args (optional) Arguments to bind to the query.
-- @param[type=table] options (optional) Options from `query_options`
-- for this query.
function _Host:iterate(query, args, options)
  return page_iterator(self, query, args, get_opts(options))
end

--- Get tracing information.
-- Retrieves the tracing information of a query (if tracing was enabled in
-- its options) from its tracing id.
--
-- @usage
-- local cassandra = require "cassandra"
-- local client = cassandra.new()
-- assert(client:connect())
--
-- local res = assert(client:execute("INSERT INTO users(id, age) VALUES(1, 33)", nil, {
--   tracing = true
-- }))
--
-- local trace = assert(client:get_trace(res.tracing_id))
-- print(trace.client) -- "127.0.0.1"
-- print(trace.command) -- "QUERY"
--
-- @param[type=string] tracing_id The query's tracing is as returned in the
-- results of a traced query.
-- @treturn table `trace`: Table holding the query's tracing events if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
function _Host:get_trace(tracing_id)
  local uuid_tracing_id = _Host.uuid(tracing_id)

  local rows, err = self:execute([[
    SELECT * FROM system_traces.sessions WHERE session_id = ?
  ]], {uuid_tracing_id})
  if not rows then return nil, 'could not get trace: '..err
  elseif #rows == 0 then return nil, 'no trace with id: '..tracing_id end

  local trace = rows[1]

  trace.events, err = self:execute([[
    SELECT * FROM system_traces.events WHERE session_id = ?
  ]], {uuid_tracing_id})
  if not trace.events then return nil, 'could not get trace events: '..err end

  return trace
end

function _Host:__tostring()
  return '<Cassandra socket: '..tostring(self.sock)..'>'
end

--- CQL serializers.
-- When binding arguments to a query, some types cannot be infered
-- automatically and will require manual serialization. Some other
-- times, it can be useful to manually enforce the type of a parameter.
-- For this purpose, shorthands for type serialization are available
-- on the `cassandra` module.
--
-- @usage
-- local cassandra = require "cassandra"
-- -- connect client...
--
-- client:execute("SELECT * FROM users WHERE id = ?", {
--   cassandra.uuid("123e4567-e89b-12d3-a456-426655440000")
-- })
--
-- client:execute("INSERT INTO users(id, emails) VALUES(?, ?)", {
--   1,
--   cassandra.set({"john@foo.com", "john@bar.com"})
-- })
--
-- @field null (native protocol v4 only) Equivalent to the `null` CQL value.
-- Useful to unset a field.
--     cassandra.null
-- @field unset Equivalent to the `not set` CQL value. Leaves field untouched
-- for binary protocol v4+, or unset it for v2/v3.
--     cassandra.unset
-- @field uuid Serialize a 32 lowercase characters string to a CQL uuid.
--     cassandra.uuid("123e4567-e89b-12d3-a456-426655440000")
-- @field timestamp Serialize a 10 digits number into a CQL timestamp.
--     cassandra.timestamp(1405356926)
-- @field list
--     cassandra.list({"abc", "def"})
-- @field set
--     cassandra.set({"abc", "def"})
-- @field map
--     cassandra.map({foo = "bar"})
-- @field udt CQL UDT.
-- @field tuple CQL tuple.
-- @field inet CQL inet.
--     cassandra.inet("127.0.0.1")
--     cassandra.inet("2001:0db8:85a3:0042:1000:8a2e:0370:7334")
-- @field bigint CQL bigint.
--     cassandra.bigint(42000000000)
-- @field double CQL double.
--     cassandra.bigint(1.0000000000000004)
-- @field ascii CQL ascii.
-- @field blob CQL blob.
-- @field boolean CQL boolean.
--     cassandra.boolean(true)
-- @field counter CQL counter.
--     cassandra.counter(1)
-- @field decimal CQL decimal.
-- @field float CQL float.
--     cassandra.float(1.618033)
-- @field int CQL int.
--     cassandra.int(10)
-- @field text CQL text.
--     cassandra.text("hello world")
-- @field timeuuid CQL timeuuid.
-- @field varchar CQL varchar.
-- @field varint CQL varint.
-- @table type_serializers

for cql_t_name, cql_t in pairs(cql.types) do
  _Host[cql_t_name] = function(val)
    if val == nil then
      error('bad argument #1 to \''..cql_t_name..'()\' (got nil)', 2)
    end
    return {val = val, __cql_type = cql_t}
  end
end

_Host.unset = cql.t_unset
_Host.null = cql.t_null

return _Host
