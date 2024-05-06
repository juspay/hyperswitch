--- Cassandra cluster client module.
-- Cluster module for OpenResty.
-- @module resty.cassandra.cluster
-- @author thibaultcha
-- @release 1.5.2

local resty_lock = require 'resty.lock'
local cassandra = require 'cassandra'
local cql = require 'cassandra.cql'

local update_time = ngx.update_time
local cql_errors = cql.errors
local requests = cql.requests
local tonumber = tonumber
local concat = table.concat
local shared = ngx.shared
local assert = assert
local pairs = pairs
local fmt = string.format
local sub = string.sub
local find = string.find
local gmatch = string.gmatch
local now = ngx.now
local type = type
local log = ngx.log
local ERR = ngx.ERR
local WARN = ngx.WARN
local DEBUG = ngx.DEBUG
local NOTICE = ngx.NOTICE

local empty_t = {}
local _log_prefix = '[lua-cassandra] '
local _rec_key = 'host:rec:'
local _prepared_key = 'prepared:id:'
local _topo_version_key = 'topo:'
local _refresh_lock_key = 'refresh:'

local function get_now()
  return now() * 1000
end

-----------------------------------------
-- Hosts status+info stored in shm
-----------------------------------------

local function set_peer(self, host, up, reconn_delay, unhealthy_at,
                        data_center, connect_err, release_version, add)
  data_center = data_center or ''
  connect_err = connect_err or ''
  release_version = release_version or ''

  local method = add and 'safe_add' or 'safe_set'

  -- host status
  local ok, err = self.shm[method](self.shm, host, up)
  if not ok and err ~= "exists" then
    return nil, 'could not set host details in shm: '..err
  end

  -- host info
  local marshalled = fmt("%d:%d:%d:%d:%s%s%s", reconn_delay, unhealthy_at,
                         #data_center, #connect_err, data_center, connect_err,
                         release_version)

  ok, err = self.shm[method](self.shm, _rec_key..host, marshalled)
  if not ok and err ~= "exists" then
    return nil, 'could not set host details in shm: '..err
  end

  return true
end

local function add_peer(self, host, up, reconn_delay, unhealthy_at,
                        data_center, connect_err, release_version)
  return set_peer(self, host, up, reconn_delay, unhealthy_at, data_center, nil,
                  release_version, true)
end

local function get_peer(self, host, status)
  local marshalled, err = self.shm:get(_rec_key .. host)
  if err then
    return nil, 'could not get host details in shm: '..err
  elseif marshalled == nil then
    local tb = debug.traceback("")
    return nil, 'no host details for '..host.."\n"..tb
  elseif type(marshalled) ~= 'string' then
    return nil, 'corrupted shm'
  end

  if status == nil then
    status, err = self.shm:get(host)
    if err then return nil, 'could not get host status in shm: '..err end
  end

  local sep_1 = find(marshalled, ":", 1, true)
  local sep_2 = find(marshalled, ":", sep_1 + 1, true)
  local sep_3 = find(marshalled, ":", sep_2 + 1, true)
  local sep_4 = find(marshalled, ":", sep_3 + 1, true)

  local reconn_delay    = sub(marshalled, 1, sep_1 - 1)
  local unhealthy_at    = sub(marshalled, sep_1 + 1, sep_2 - 1)
  local data_center_len = sub(marshalled, sep_2 + 1, sep_3 - 1)
  local err_len         = sub(marshalled, sep_3 + 1, sep_4 - 1)

  local data_center_last = sep_4 + tonumber(data_center_len)
  local err_last = data_center_last + tonumber(err_len)

  local data_center     = sub(marshalled, sep_4 + 1, data_center_last)
  local err_conn        = sub(marshalled, data_center_last + 1, err_last)
  local release_version = sub(marshalled, err_last + 1)

  return {
    up = status,
    host = host,
    data_center = data_center ~= '' and data_center or nil,
    release_version = release_version ~= '' and release_version or nil,
    reconn_delay = tonumber(reconn_delay),
    unhealthy_at = tonumber(unhealthy_at),
    err = err_conn,
  }
end

local function set_peers(self, topo_version, peers, protocol_version)
  local marshalled = {}

  for i = 1, #peers do
    marshalled[i] = peers[i].host
  end

  marshalled = concat(marshalled, ",")

  if protocol_version then
    marshalled = protocol_version .. "|" .. marshalled
  end

  local ok, err = self.shm:safe_set(_topo_version_key .. topo_version,
                                    marshalled)
  if not ok then return nil, 'could not set peers in shm: '..err end

  return true
end

local function get_peers(self, topo_version)
  if not topo_version then
    topo_version = self.shm:get(_topo_version_key .. 'latest')
    if not topo_version then return end
  end

  local marshalled, err = self.shm:get(_topo_version_key .. topo_version)
  if err then return nil, 'could not get peers from shm: '..err
  elseif not marshalled then return end

  local peers = {
    topology_version = topo_version,
  }

  local sep_1 = find(marshalled, "|", 1, true)
  if sep_1 then
    peers.protocol_version = tonumber(sub(marshalled, 1, sep_1 - 1), 10)
    marshalled = sub(marshalled, sep_1 + 1)
  end

  for host in gmatch(marshalled, '([^,]+)') do
    local peer, err = get_peer(self, host)
    if not peer then
      return nil, 'could not get peer from shm: '..err
    end

    peers[#peers + 1] = peer
  end

  return peers
end

local function delete_peer(self, host)
  self.shm:delete(_rec_key .. host) -- details
  self.shm:delete(host) -- status bool
end

local function set_peer_down(self, host, connect_err)
  if self.logging then
    log(WARN, _log_prefix, 'setting host at ', host, ' DOWN')
  end

  local peer = get_peer(self, host, false)
  peer = peer or empty_t -- this can be called from refresh() so no host in shm yet

  return set_peer(self, host, false, self.reconn_policy:next_delay(host), get_now(),
                  peer.data_center, connect_err, peer.release_version)
end

local function set_peer_up(self, host)
  if self.logging then
    log(NOTICE, _log_prefix, 'setting host at ', host, ' UP')
  end
  self.reconn_policy:reset(host)

  local peer = get_peer(self, host, true)
  peer = peer or empty_t -- this can be called from refresh() so no host in shm yet

  return set_peer(self, host, true, 0, 0,
                  peer.data_center, nil, peer.release_version)
end

local function can_try_peer(self, host)
  local up, err = self.shm:get(host)
  if up then return up
  elseif err then return nil, err
  else
    -- reconnection policy steps in before making a decision
    local peer_rec, err = get_peer(self, host, up)
    if not peer_rec then return nil, err end
    return get_now() - peer_rec.unhealthy_at >= peer_rec.reconn_delay,
           nil, true, peer_rec
  end
end

----------------------------
-- utils
----------------------------

local function spawn_peer(host, port, keyspace, opts)
  opts = opts or {}
  opts.host = host
  opts.port = port
  opts.keyspace = keyspace
  return cassandra.new(opts)
end

local function check_peer_health(self, host, coordinator_options, retry)
  coordinator_options = coordinator_options or empty_t

  local keyspace
  if not coordinator_options.no_keyspace then
    keyspace = coordinator_options.keyspace or self.keyspace
  end

  local peer, err = spawn_peer(host, self.default_port, keyspace, self.peers_opts)
  if not peer then return nil, err
  else
    peer:settimeout(self.timeout_connect)
    local ok, err_conn, maybe_down = peer:connect()
    if ok then
      -- host is healthy
      if retry then
        -- node seems healthy after being down, back up!
        local ok, err = set_peer_up(self, host)
        if not ok then return nil, 'error setting host back up: '..err end
      end

      peer:settimeout(self.timeout_read)

      return peer
    elseif maybe_down then
      -- host is not (or still not) responsive
      local ok, shm_err = set_peer_down(self, host, err_conn)
      if not ok then return nil, 'error setting host down: '..shm_err end

      return nil, 'host seems unhealthy, considering it down ('..err_conn..')'
    else
      return nil, err_conn
    end
  end
end

-----------
-- Cluster
-----------

local _Cluster = {
  _VERSION = '1.5.2',
  _log_prefix = _log_prefix,
}

_Cluster.__index = _Cluster

--- New cluster options.
-- Options taken by `new` upon cluster creation.
-- @field shm Name of the lua_shared_dict to use for this cluster's
-- information. (`string`, default: `cassandra`)
-- @field contact_points Array of addresses for this cluster's
-- contact points. (`table`, default: `{"127.0.0.1"}`)
-- @field default_port The port on which all nodes from the cluster are
-- listening on. (`number`, default: `9042`)
-- @field keyspace Keyspace to use for this cluster. (`string`, optional)
-- @field timeout_connect The timeout value when connecing to a node, in ms.
-- (`number`, default: `1000`)
-- @field timeout_read The timeout value when reading from a node, in ms.
-- (`number`, default: `2000`)
-- @field retry_on_timeout Specifies if the request should be retried on the
-- next coordinator (as per the load balancing policy)
-- if it timed out. (`boolean`, default: `true`)
-- @field max_schema_consensus_wait Maximum waiting time allowed when executing
-- DDL queries before timing out, in ms.
-- (`number`, default: `10000`)
-- @field lock_timeout Timeout value of lua-resty-lock used for the `refresh`
-- and prepared statement mutexes, in seconds.
-- (`number`, optional)
-- @field silent Disables all logging (of any log_level) from this cluster.
-- (`boolean`, default: `false`)
-- @field lb_policy A load balancing policy created from one of the modules
-- under `resty.cassandra.policies.lb.*`.
-- (`lb policy`, default: `lb.rr` round robin)
-- @field reconn_policy A reconnection policy created from one of the modules
-- under `resty.cassandra.policies.reconnection.*`.
-- (`reconn policy`, default: `reconnection.exp` (exponential)
-- 1000ms base, 60000ms max)
-- @field retry_policy A retry policy created from one of the modules
-- under `resty.cassandra.policies.retry.*`.
-- (`retry policy`, default: `retry.simple`, 3 retries)
-- @field ssl Determines if the created cluster should connect using SSL.
-- (`boolean`, default: `false`)
-- @field verify Enable server certificate validation if `ssl` is enabled.
-- (`boolean`, default: `false`)
-- @field auth Authentication handler, created from the
-- `cassandra.auth_providers` table. (optional)
-- @table `cluster_options`

--- Create a new Cluster client.
-- Takes a table of `cluster_options`. Does not connect automatically.
-- On the first request to the cluster, the module will attempt to connect to
-- one of the specified `contact_points`, and retrieve the full list of nodes
-- belonging to this cluster. Once this list retrieved, the load balancing
-- policy will start selecting nodes to act as coordinators for the future
-- requests.
--
-- @usage
-- local Cluster = require "resty.cassandra.cluster"
-- local cluster = Cluster.new {
--   shm = "cassandra_shared_dict",
--   contact_points = {"10.0.0.1", "10.0.0.2"},
--   keyspace = "my_keyspace",
--   default_port = 9042,
--   timeout_connect = 3000
-- }
--
-- @param[type=table] opts Options for the created cluster client.
-- @treturn table `cluster`: A table holding clustering operations capabilities
-- or nil if failure.
-- @treturn string `err`: String describing the error if failure.
function _Cluster.new(opts)
  opts = opts or empty_t
  if type(opts) ~= 'table' then
    return nil, 'opts must be a table'
  end

  local peers_opts = {}
  local lock_opts = {}
  local dict_name = opts.shm or 'cassandra'
  if type(dict_name) ~= 'string' then
    return nil, 'shm must be a string'
  elseif not shared[dict_name] then
    return nil, 'no shared dict '..dict_name
  end

  for k, v in pairs(opts) do
    if k == 'keyspace' then
      if type(v) ~= 'string' then
        return nil, 'keyspace must be a string'
      end
    elseif k == 'ssl' then
      if type(v) ~= 'boolean' then
        return nil, 'ssl must be a boolean'
      end
      peers_opts.ssl = v
    elseif k == 'verify' then
      if type(v) ~= 'boolean' then
        return nil, 'verify must be a boolean'
      end
      peers_opts.verify = v
    elseif k == 'cafile' then
      if type(v) ~= 'string' then
        return nil, 'cafile must be a string'
      end
      peers_opts.cafile = v
    elseif k == 'ssl_protocol' then
      if type(v) ~= 'string' then
        return nil, 'ssl_protocol must be a string'
      end
      peers_opts.ssl_protocol = v
    elseif k == 'auth' then
      if type(v) ~= 'table' then
        return nil, 'auth seems not to be an auth provider'
      end
      peers_opts.auth = v
    elseif k == 'default_port' then
      if type(v) ~= 'number' then
        return nil, 'default_port must be a number'
      end
    elseif k == 'contact_points' then
      if type(v) ~= 'table' then
        return nil, 'contact_points must be a table'
      end
    elseif k == 'timeout_read' then
      if type(v) ~= 'number' then
        return nil, 'timeout_read must be a number'
      end
    elseif k == 'timeout_connect' then
      if type(v) ~= 'number' then
        return nil, 'timeout_connect must be a number'
      end
    elseif k == 'max_schema_consensus_wait' then
      if type(v) ~= 'number' then
        return nil, 'max_schema_consensus_wait must be a number'
      end
    elseif k == 'retry_on_timeout' then
      if type(v) ~= 'boolean' then
        return nil, 'retry_on_timeout must be a boolean'
      end
    elseif k == 'lock_timeout' then
      if type(v) ~= 'number' then
        return nil, 'lock_timeout must be a number'
      end
      lock_opts.timeout = v
    elseif k == 'silent' then
      if type(v) ~= 'boolean' then
        return nil, 'silent must be a boolean'
      end
    end
  end

  return setmetatable({
    topo_ver = 0,
    shm = shared[dict_name],
    dict_name = dict_name,
    prepared_ids = {},
    peers_opts = peers_opts,
    keyspace = opts.keyspace,
    default_port = opts.default_port or 9042,
    contact_points = opts.contact_points or {'127.0.0.1'},
    timeout_read = opts.timeout_read or 2000,
    timeout_connect = opts.timeout_connect or 1000,
    retry_on_timeout = opts.retry_on_timeout == nil and true or opts.retry_on_timeout,
    max_schema_consensus_wait = opts.max_schema_consensus_wait or 10000,
    lock_opts = lock_opts,
    logging = not opts.silent,

    lb_policy = opts.lb_policy
                or require('resty.cassandra.policies.lb.rr').new(),
    reconn_policy = opts.reconn_policy
                or require('resty.cassandra.policies.reconnection.exp').new(1000, 60000),
    retry_policy = opts.retry_policy
                or require('resty.cassandra.policies.retry.simple').new(3),
  }, _Cluster)
end

local function no_host_available_error(errors)
  local buf = {'all hosts tried for query failed'}
  for address, err in pairs(errors) do
    buf[#buf+1] = address..': '..err
  end
  return concat(buf, '. ')
end

local function first_coordinator(self)
  local errors = {}
  local cp = self.contact_points

  for i = 1, #cp do
    local peer, err = check_peer_health(self, cp[i], {
      no_keyspace = true,
    })
    if not peer then
      errors[cp[i]] = err
    else
      return peer, nil, cp[i]
    end
  end

  return nil, no_host_available_error(errors)
end

local function next_coordinator(self, coordinator_options)
  local errors = {}

  for _, peer_rec in self.lb_policy:iter() do
    local ok, err, retry, peer_state = can_try_peer(self, peer_rec.host)
    if ok then
      local peer, err = check_peer_health(self, peer_rec.host, coordinator_options, retry)
      if peer then
        if self.logging then
          log(DEBUG, _log_prefix, 'load balancing policy chose host at ',  peer.host)
        end
        return peer
      else
        errors[peer_rec.host] = err
      end
    elseif err then
      return nil, err
    else
      local s = 'host still considered down'
      if peer_state then
        local waited = get_now() - peer_state.unhealthy_at
        s = s .. ' for ' .. (peer_state.reconn_delay - waited) / 1000 .. 's'

        if peer_state.err and peer_state.err ~= '' then
          s = s .. ' (last error: ' .. peer_state.err .. ')'
        else
          s = s .. ' (last error: not recorded)'
        end
      end

      errors[peer_rec.host] = s
    end
  end

  return nil, no_host_available_error(errors)
end

local function compare_peers(t1, t2, tc)
  for i = 1, #t1 do
    local found

    for j = 1, #t2 do
      if t1[i].host == t2[j].host then
        found = true
        break
      end
    end

    if not found then
      table.insert(tc, t1[i].host)
    end
  end
end

local function err_with_unlock(lock, err)
  local ok, unlock_err = lock:unlock()
  if not ok then
    err = err ..  ' (failed to unlock refresh lock: '..unlock_err..')'
  end
  return nil, err
end

--- Refresh the list of nodes in the cluster.
-- Queries one of the specified `contact_points` to retrieve the list of
-- available nodes in the cluster, and update the configured policies.
-- The query will use the timeout threshold specified in the `read_timeout`
-- option of the `new` method.
-- This method is safe be called at runtime, by multiple workers at the same
-- time, which can be useful to refresh the cluster topology when nodes are
-- added or removed from the cluster.
-- This method is automatically called upon the first query made to the
-- cluster (from `execute`, `batch` or `iterate`), but needs to be manually
-- called if further updates are required.
-- @param[type=number] timeout Timeout threshold (in seconds) for a given
-- worker when another worker is already refreshing the topology (defaults to
-- the `lock_timeout` option of the `new` method).
-- @treturn boolean `ok`: `true` if success, `nil` if failure.
-- @treturn string `err`: String describing the error if failure.
-- @treturn table `topology`: A table containing the topology changes if any.
-- This value will only be returned when the worker acquired the lock.
function _Cluster:refresh(timeout)
  local ver_topo, ver_refresh = self.shm:get(_topo_version_key .. 'latest')
  if not ver_topo then
    ver_topo = 0
    ver_refresh = 0
  end

  local topo_changes

  if self.topo_ver == ver_topo then
    -- we already have the latest known cluster topology, try to
    -- acquire lock to fetch a new one

    if not timeout then
      timeout = self.lock_opts.timeout
    end

    log(DEBUG, _log_prefix, 'refresh: attempting to acquire lock...',
               ' (ver_refresh=', ver_refresh, ', timeout=', timeout, ')')

    local lock = resty_lock:new(self.dict_name, {
      timeout = timeout,
      exptime = timeout and timeout + 1
    })
    local elapsed, err = lock:lock(_refresh_lock_key .. ver_refresh)
    if not elapsed then
      return nil, 'failed to acquire refresh lock: '..err..
                  ' (ver_refresh='..ver_refresh..')'
    end

    if elapsed == 0 then
      ver_refresh = ver_refresh + 1

      log(DEBUG, _log_prefix, 'refresh: lock acquired, fetching topology...',
                 ' (ver_refresh=', ver_refresh, ')')

      local coordinator, err, local_cp = first_coordinator(self)
      if not coordinator then
        return err_with_unlock(lock, err)
      end

      coordinator:settimeout(self.timeout_read)

      local local_rows, err = coordinator:execute [[
        SELECT data_center,rpc_address,release_version FROM system.local
      ]]
      if not local_rows then
        return err_with_unlock(lock, err)
      end

      if local_rows[1] == nil then
        assert(err_with_unlock(lock, 'local host could not be found'))
      end

      local rows, err = coordinator:execute [[
        SELECT peer,data_center,rpc_address,release_version FROM system.peers
      ]]
      if not rows then
        return err_with_unlock(lock, err)
      end

      coordinator:setkeepalive()

      local local_addr = local_rows[1].rpc_address
      if local_addr == "0.0.0.0" or local_addr == "::" then
        if self.logging then
          log(WARN, _log_prefix, 'found contact point with \'', local_addr, '\' ',
                                 'as rpc_address, using \'', local_cp, '\' to ',
                                 'contact it instead. If this is incorrect ',
                                 'you should avoid using \'', local_addr, '\' ',
                                 'in rpc_address')
        end

        local_addr = local_cp
      end

      rows[#rows+1] = { -- local host
        rpc_address = local_addr,
        data_center = local_rows[1].data_center,
        release_version = local_rows[1].release_version
      }

      log(DEBUG, _log_prefix, 'refresh: cluster topology fetched ',
                 '(ver_refresh=', ver_refresh, ', n_peers=', #rows, ')')

      for i = 1, #rows do
        if rows[i].rpc_address then
          if rows[i].rpc_address == "0.0.0.0" or rows[i].rpc_address == "::" then
            if self.logging then
              log(WARN, _log_prefix, 'found host with \'', rows[i].rpc_address, '\',',
                                     ' as rpc_address, using \'', rows[i].peer, '\'',
                                     ' to contact it instead. If this is ',
                                     'incorrect you should avoid using \'',
                                     rows[i].rpc_address, '\' in rpc_address')
            end

            rows[i].host = rows[i].peer
          else
            rows[i].host = rows[i].rpc_address
          end
        end
      end

      topo_changes = {
        added = {},
        removed = {},
      }

      if ver_refresh == 1 then
        for i = 1, #rows do
          table.insert(topo_changes.added, rows[i].host)
        end
      else
        local old_peers, err = get_peers(self, ver_topo)
        if err then return err_with_unlock(lock, err)
        elseif not old_peers then
          log(WARN, _log_prefix, 'refresh: missing peers entry when comparing ',
                    'topologies (ver_refresh=', ver_refresh, ')')
        else
          compare_peers(rows, old_peers, topo_changes.added)
          compare_peers(old_peers, rows, topo_changes.removed)
        end
      end

      local rebuild = #topo_changes.added > 0 or #topo_changes.removed > 0

      log(DEBUG, _log_prefix, 'refresh: changes detected in topology: ',
                 rebuild and 'yes' or 'no', ' (ver_refresh=', ver_refresh, ')')

      if rebuild then
        ver_topo = ver_topo + 1

        for i = 1, #rows do
          if not rows[i].rpc_address then
            log(ERR, _log_prefix, 'no rpc_address found for host ', rows[i].peer,
                                  ' in ', coordinator.host, '\'s peers system ',
                                  'table. ', rows[i].peer, ' will be ignored.')
          else
            local ok, err = add_peer(self, rows[i].host, true, 0, 0,
                                     rows[i].data_center, nil,
                                     rows[i].release_version)
            if not ok then return err_with_unlock(lock, err) end
          end
        end

        local ok, err = set_peers(self, ver_topo, rows,
                                  coordinator.protocol_version)
        if not ok then return err_with_unlock(lock, err) end
      end

      local ok, err = self.shm:set(_topo_version_key .. 'latest', ver_topo, 0, ver_refresh)
      if not ok then return err_with_unlock(lock, 'failed to set topo and refresh versions: '..err) end
    end

    local ok, err = lock:unlock()
    if not ok then return nil, 'failed to unlock refresh lock: '..err end

    ver_topo = self.shm:get(_topo_version_key .. 'latest')
  elseif self.topo_ver < ver_topo then
    log(DEBUG, _log_prefix, 'refresh: cluster topology already fetched, ',
               'rebuilding policies (ver_topo=', ver_topo, ')')
  elseif self.topo_ver > ver_topo then
    log(WARN, _log_prefix, 'refresh: cluster topology version ahead,',
              ' rebuilding policies (cluster.topo_ver=', self.topo_ver, ')')
  end

  if ver_topo ~= self.topo_ver then
    local peers, err = get_peers(self, ver_topo)
    if err then return nil, err
    elseif not peers then return nil, 'no peers for topology version: ' .. ver_topo end

    -- setting protocol_version early so we don't always attempt a connection
    -- with an incompatible one, triggerring more round trips
    self.peers_opts.protocol_version = peers.protocol_version

    -- initiate the load balancing policy
    self.lb_policy:init(peers)

    self.topo_ver = ver_topo

    log(DEBUG, _log_prefix, 'refresh: cluster topology refreshed: yes',
               ' (ver_refresh=', ver_refresh, ', ver_topo=', ver_topo, ')')
  else
    log(DEBUG, _log_prefix, 'refresh: cluster topology refreshed: no',
               ' (ver_refresh=', ver_refresh, ', ver_topo=', ver_topo, ')')
  end

  return true, nil, topo_changes
end

--------------------
-- queries execution
--------------------

local function check_schema_consensus(coordinator)
  local local_res, err = coordinator:execute('SELECT schema_version FROM system.local')
  if not local_res then return nil, err end

  local peers_res, err = coordinator:execute('SELECT schema_version FROM system.peers')
  if not peers_res then return nil, err end

  if #peers_res > 0 and #local_res > 0 then
    for i = 1, #peers_res do
      if peers_res[i].schema_version ~= local_res[1].schema_version then
        return nil
      end
    end
  end

  return local_res[1].schema_version
end

local function wait_schema_consensus(self, coordinator, timeout)
  timeout = timeout or self.max_schema_consensus_wait
  local peers, err = get_peers(self)
  if err then return nil, err
  elseif not peers then return nil, 'no peers in shm'
  elseif #peers < 2 then return true end

  update_time()

  local ok, err, tdiff
  local tstart = get_now()

  repeat
    -- disabled because this method is currently used outside of an
    -- ngx_lua compatible context by production applications.
    -- no fallback implemented yet.
    --ngx.sleep(0.5)

    update_time()
    ok, err = check_schema_consensus(coordinator)
    tdiff = get_now() - tstart
  until ok or err or tdiff >= timeout

  if ok then
    return ok
  elseif err then
    return nil, err
  else
    return nil, 'timeout'
  end
end

local function prepare(self, coordinator, query)
  if self.logging then
    log(DEBUG, _log_prefix, 'preparing ', query, ' on host ', coordinator.host)
  end
  -- we are the ones preparing the query
  local res, err = coordinator:prepare(query)
  if not res then return nil, 'could not prepare query: '..err end
  return res.query_id
end

local function get_or_prepare(self, coordinator, query)
  -- worker memory check
  local query_id = self.prepared_ids[query]
  if not query_id then
    -- worker cache miss
    -- shm cache?
    local shm = self.shm
    local key = _prepared_key .. query
    local err
    query_id, err = shm:get(key)
    if err then return nil, 'could not get query id from shm:'..err
    elseif not query_id then
      -- shm cache miss
      -- query not prepared yet, must prepare in mutex
      local lock = resty_lock:new(self.dict_name, self.lock_opts)
      local elapsed, err = lock:lock('prepare:' .. query)
      if not elapsed then return nil, 'failed to acquire lock: '..err end

      -- someone else prepared query?
      query_id, err = shm:get(key)
      if err then return nil, 'could not get query id from shm:'..err
      elseif not query_id then
        query_id, err = prepare(self, coordinator, query)
        if not query_id then return nil, err end

        local ok, err = shm:safe_set(key, query_id)
        if not ok then
          if err == 'no memory' then
            log(WARN, _log_prefix, 'could not set query id in shm: ',
                      'running out of memory, please increase the ',
                      self.dict_name, ' dict size')
          else
            return nil, 'could not set query id in shm: '..err end
          end
      end

      local ok, err = lock:unlock()
      if not ok then return nil, 'failed to unlock: '..err end
    end

    -- set worker cache
    self.prepared_ids[query] = query_id
  end

  return query_id
end

local send_request

function _Cluster:send_retry(request, ...)
  local coordinator, err = next_coordinator(self)
  if not coordinator then return nil, err end

  if self.logging then
    log(NOTICE, _log_prefix, 'retrying request on host at ', coordinator.host,
                             ' reason: ', ...)
  end

  request.retries = request.retries + 1

  return send_request(self, coordinator, request)
end

local function prepare_and_retry(self, coordinator, request)
  if request.queries then
    -- prepared batch
    if self.logging then
      log(NOTICE, _log_prefix, 'some requests from this batch were not prepared on host ',
                  coordinator.host, ', preparing and retrying')
    end
    for i = 1, #request.queries do
      local query_id, err = prepare(self, coordinator, request.queries[i][1])
      if not query_id then return nil, err end
      request.queries[i][3] = query_id
    end
  else
    -- prepared query
    if self.logging then
      log(NOTICE, _log_prefix, request.query, ' was not prepared on host ',
                  coordinator.host, ', preparing and retrying')
    end
    local query_id, err = prepare(self, coordinator, request.query)
    if not query_id then return nil, err end
    request.query_id = query_id
  end

  return send_request(self, coordinator, request)
end

local function handle_error(self, err, cql_code, coordinator, request)
  if cql_code and cql_code == cql_errors.UNPREPARED then
    return prepare_and_retry(self, coordinator, request)
  end

  -- failure, need to try another coordinator
  coordinator:setkeepalive()

  if cql_code then
    local retry
    if cql_code == cql_errors.OVERLOADED or
       cql_code == cql_errors.IS_BOOTSTRAPPING or
       cql_code == cql_errors.TRUNCATE_ERROR then
      retry = true
    elseif cql_code == cql_errors.UNAVAILABLE_EXCEPTION then
      retry = self.retry_policy:on_unavailable(request)
    elseif cql_code == cql_errors.READ_TIMEOUT then
      retry = self.retry_policy:on_read_timeout(request)
    elseif cql_code == cql_errors.WRITE_TIMEOUT then
      retry = self.retry_policy:on_write_timeout(request)
    end

    if retry then
      return self:send_retry(request, 'CQL code: ', cql_code)
    end
  elseif err == 'timeout' then
    if self.retry_on_timeout then
      return self:send_retry(request, 'timeout')
    end
  else
    -- host seems down?
    local ok, err2 = set_peer_down(self, coordinator.host, err)
    if not ok then return nil, err2 end
    return self:send_retry(request, 'coordinator seems down (' .. err .. ')')
  end

  return nil, err, cql_code
end

send_request = function(self, coordinator, request)
  local res, err, cql_code = coordinator:send(request)
  if not res then
    return handle_error(self, err, cql_code, coordinator, request)
  elseif res.warnings and self.logging then
    -- protocol v4 can return warnings to the client
    for i = 1, #res.warnings do
      log(WARN, _log_prefix, res.warnings[i])
    end
  end

  if res.type == 'SCHEMA_CHANGE' then
    local schema_version, err = wait_schema_consensus(self, coordinator)
    if not schema_version then
      coordinator:setkeepalive()
      return nil, 'could not check schema consensus: '..err
    end

    res.schema_version = schema_version
  end

  coordinator:setkeepalive()

  return res
end

do
  local get_request_opts = cassandra.get_request_opts
  local page_iterator = cassandra.page_iterator
  local query_req = requests.query.new
  local batch_req = requests.batch.new
  local prep_req = requests.execute_prepared.new

  --- Coordinator options.
  -- Options to pass to coordinators chosen by the load balancing policy
  -- on `execute`/`batch`/`iterate`.
  -- @field keyspace Keyspace to use for the current request connection.
  -- (`string`, optional)
  -- @field no_keyspace Does not set a keyspace for the current request
  -- connection.
  -- (`boolean`, default: `false`)
  -- @table `coordinator_options`

  --- Execute a query.
  -- Sends a request to the coordinator chosen by the configured load
  -- balancing policy. The policy always chooses nodes that are considered
  -- healthy, and eventually reconnects to unhealthy nodes as per the
  -- configured reconnection policy.
  -- Requests that fail because of timeouts can be retried on the next
  -- available node if `retry_on_timeout` is enabled, and failed requests
  -- can be retried as per defined in the configured retry policy.
  --
  -- @usage
  -- local Cluster = require "resty.cassandra.cluster"
  -- local cluster, err = Cluster.new()
  -- if not cluster then
  --   ngx.log(ngx.ERR, "could not create cluster: ", err)
  --   ngx.exit(500)
  -- end
  --
  -- local rows, err = cluster:execute("SELECT * FROM users WHERE age = ?". {
  --   21
  -- }, {
  --   page_size = 100
  -- })
  -- if not rows then
  --   ngx.log(ngx.ERR, "could not retrieve users: ", err)
  --   ngx.exit(500)
  -- end
  --
  -- ngx.say("page size: ", #rows, " next page: ", rows.meta.paging_state)
  --
  -- @param[type=string] query CQL query to execute.
  -- @param[type=table] args (optional) Arguments to bind to the query.
  -- @param[type=table] options (optional) Options from `query_options`.
  -- @param[type=table] coordinator_options (optional) Options from `coordinator_options`
  -- for this query.
  -- @treturn table `res`: Table holding the query result if success, `nil` if failure.
  -- @treturn string `err`: String describing the error if failure.
  -- @treturn number `cql_err`: If a server-side error occurred, the CQL error code.
  function _Cluster:execute(query, args, options, coordinator_options)
    if self.topo_ver == 0 then
      local ok, err = self:refresh()
      if not ok then return nil, 'could not refresh cluster: '..err end
    end

    coordinator_options = coordinator_options or empty_t

    local coordinator, err = next_coordinator(self, coordinator_options)
    if not coordinator then return nil, err end

    local request
    local opts = get_request_opts(options)

    if opts.prepared then
      local query_id, err = get_or_prepare(self, coordinator, query)
      if not query_id then return nil, err end
      request = prep_req(query_id, args, opts, query)
    else
      request = query_req(query, args, opts)
    end

    return send_request(self, coordinator, request)
  end

  --- Execute a batch.
  -- Sends a request to execute the given batch. Load balancing, reconnection,
  -- and retry policies act the same as described for `execute`.
  -- @usage
  -- local Cluster = require "resty.cassandra.cluster"
  -- local cluster, err = Cluster.new()
  -- if not cluster then
  --   ngx.log(ngx.ERR, "could not create cluster: ", err)
  --   ngx.exit(500)
  -- end
  --
  -- local res, err = cluster:batch({
  --   {"INSERT INTO things(id, n) VALUES(?, 1)", {123}},
  --   {"UPDATE things SET n = 2 WHERE id = ?", {123}},
  --   {"UPDATE things SET n = 3 WHERE id = ?", {123}}
  -- }, {
  --   logged = false
  -- })
  -- if not res then
  --   ngx.log(ngx.ERR, "could not execute batch: ", err)
  --   ngx.exit(500)
  -- end
  --
  -- @param[type=table] queries CQL queries to execute.
  -- @param[type=table] options (optional) Options from `query_options`.
  -- @param[type=table] coordinator_options (optional) Options from `coordinator_options`
  -- for this query.
  -- @treturn table `res`: Table holding the query result if success, `nil` if failure.
  -- @treturn string `err`: String describing the error if failure.
  -- @treturn number `cql_err`: If a server-side error occurred, the CQL error code.
  function _Cluster:batch(queries, options, coordinator_options)
    if self.topo_ver == 0 then
      local ok, err = self:refresh()
      if not ok then return nil, 'could not refresh cluster: '..err end
    end

    coordinator_options = coordinator_options or empty_t

    local coordinator, err = next_coordinator(self, coordinator_options)
    if not coordinator then return nil, err end

    local opts = get_request_opts(options)

    if opts.prepared then
      for i = 1, #queries do
        local query_id, err = get_or_prepare(self, coordinator, queries[i][1])
        if not query_id then return nil, err end
        queries[i][3] = query_id
      end
    end

    return send_request(self, coordinator, batch_req(queries, opts))
  end

  --- Lua iterator for auto-pagination.
  -- Perform auto-pagination for a query when used as a Lua iterator.
  -- Load balancing, reconnection, and retry policies act the same as described
  -- for `execute`.
  --
  -- @usage
  -- local Cluster = require "resty.cassandra.cluster"
  -- local cluster, err = Cluster.new()
  -- if not cluster then
  --   ngx.log(ngx.ERR, "could not create cluster: ", err)
  --   ngx.exit(500)
  -- end
  --
  -- for rows, err, page in cluster:iterate("SELECT * FROM users") do
  --   if err then
  --     ngx.log(ngx.ERR, "could not retrieve page: ", err)
  --     ngx.exit(500)
  --   end
  --   ngx.say("page ", page, " has ", #rows, " rows")
  -- end
  --
  -- @param[type=string] query CQL query to execute.
  -- @param[type=table] args (optional) Arguments to bind to the query.
  -- @param[type=table] options (optional) Options from `query_options`
  -- for this query.
  function _Cluster:iterate(query, args, options)
    return page_iterator(self, query, args, options)
  end
end

_Cluster.set_peer = set_peer
_Cluster.get_peer = get_peer
_Cluster.add_peer = add_peer
_Cluster.get_peers = get_peers
_Cluster.set_peers = set_peers
_Cluster.delete_peer = delete_peer
_Cluster.set_peer_up = set_peer_up
_Cluster.can_try_peer = can_try_peer
_Cluster.handle_error = handle_error
_Cluster.set_peer_down = set_peer_down
_Cluster.get_or_prepare = get_or_prepare
_Cluster.next_coordinator = next_coordinator
_Cluster.first_coordinator = first_coordinator
_Cluster.wait_schema_consensus = wait_schema_consensus
_Cluster.check_schema_consensus = check_schema_consensus

return _Cluster
