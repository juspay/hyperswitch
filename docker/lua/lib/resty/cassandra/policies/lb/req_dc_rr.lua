--- Request and datacenter-aware round robin load balancing policy for OpenResty.
-- This policy will try to reuse the same node for the lifecycle of a given
-- request if possible. It is mostly designed for use in OpenResty
-- environments.
--
-- This ensures that the underlying connection pool to the node is reused as
-- much as possible. If a new node is chosen for every query, there is no
-- guarantee that each selected node will already have a pre-established
-- connection. Since this policy reuses the same node for the lifecycle of a
-- request, the chances of having to open a new connection are much reduced.
--
-- Since this policy is also datacenter-aware, it will prioritize nodes in the
-- local datacenter as well (when no node has been elected for a given request
-- yet).
-- @module resty.cassandra.policies.lb.req_dc_rr
-- @author kikito

local cluster = require "resty.cassandra.cluster"
local _M = require('resty.cassandra.policies.lb').new_policy('req_and_dc_aware_round_robin')

local past_init

--- Create a request and DC-aware round robin policy.
-- Instanciates a request and DC-aware round robin policy for
-- `resty.cassandra.cluster`.
--
-- @usage
-- local Cluster = require "resty.cassandra.cluster"
-- local req_dc_rr = require "resty.cassandra.policies.lb.req_dc_rr"
--
-- local policy = req_dc_rr.new("my_local_cluster_name")
-- local cluster = assert(Cluster.new {
--   lb_policy = policy
-- })
--
-- @param[type=string] local_dc Name of the local/closest datacenter.
-- @treturn table `policy`: A DC-aware round robin policy.
function _M.new(local_dc)
  assert(type(local_dc) == 'string', 'local_dc must be a string')

  local self = _M.super.new()
  self.local_dc = local_dc
  return self
end

function _M:init(peers)
  local local_peers, remote_peers = {}, {}

  for i = 1, #peers do
    if type(peers[i].data_center) ~= 'string' then
      ngx.log(ngx.WARN, cluster._log_prefix, 'peer ', peers[i].host,
              ' has no data_center field in shm, considering it remote')

      peers[i].data_center = nil
    end

    if self.local_dc and peers[i].data_center == self.local_dc then
      local_peers[#local_peers+1] = peers[i]

    else
      remote_peers[#remote_peers+1] = peers[i]
    end
  end

  self.start_local_idx = -2
  self.start_remote_idx = -2
  self.local_peers = local_peers
  self.remote_peers = remote_peers
end

local function advance_local_or_remote_peer(state)
  if state.local_tried < #state.local_peers then
    state.local_tried = state.local_tried + 1
    state.local_idx = state.local_idx + 1

    local peer = state.local_peers[(state.local_idx % #state.local_peers) + 1]

    if state.ctx then
      state.ctx.cassandra_coordinator = peer
    end

    return peer

  elseif state.remote_tried < #state.remote_peers then
    state.remote_tried = state.remote_tried + 1
    state.remote_idx = state.remote_idx + 1

    return state.remote_peers[(state.remote_idx % #state.remote_peers) + 1]
  end
end

local function next_peer(state, i)
  i = i + 1

  if i == 1 and state.initial_cassandra_coordinator then
    return i, state.initial_cassandra_coordinator
  end

  local peer = advance_local_or_remote_peer(state)
  if not peer then
    return nil
  end

  if peer == state.initial_cassandra_coordinator then
    peer = advance_local_or_remote_peer(state)
    if not peer then
      return nil
    end
  end

  return i + 1, peer
end

function _M:iter()
  self.local_tried = 0
  self.remote_tried = 0

  if past_init or ngx.get_phase() ~= "init" then
    self.ctx = ngx and ngx.ctx
    past_init = true
  end

  if self.ctx then
    self.initial_cassandra_coordinator = self.ctx.cassandra_coordinator
  end

  self.local_idx = (self.start_local_idx % #self.local_peers) + 1
  self.remote_idx = (self.start_remote_idx % #self.remote_peers) + 1

  self.start_remote_idx = self.start_remote_idx + 1
  self.start_local_idx = self.start_local_idx + 1

  return next_peer, self, 0
end

return _M
