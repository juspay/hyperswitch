--- Datacenter-aware round robin load balancing policy.
-- This policy will work better than its plain Round Robin counterpart
-- in multi-datacenters setups.
-- It is implemented in such a fashion that it will always prioritize nodes
-- from the local/closest datacenter (which needs to be manually specified).
-- @module resty.cassandra.policies.lb.dc_rr
-- @author thibaultcha

local cluster = require "resty.cassandra.cluster"
local _M = require('resty.cassandra.policies.lb').new_policy('dc_aware_round_robin')

--- Create a DC-aware round robin policy.
-- Instanciates a DC-aware round robin policy for `resty.cassandra.cluster`.
--
-- @usage
-- local Cluster = require "resty.cassandra.cluster"
-- local dc_rr = require "resty.cassandra.policies.lb.dc_rr"
--
-- local policy = dc_rr.new("my_local_cluster_name")
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

local function next_peer(state, i)
  i = i + 1

  if state.local_tried < #state.local_peers then
    state.local_tried = state.local_tried + 1
    state.local_idx = state.local_idx + 1

    return i, state.local_peers[(state.local_idx % #state.local_peers) + 1]

  elseif state.remote_tried < #state.remote_peers then
    state.remote_tried = state.remote_tried + 1
    state.remote_idx = state.remote_idx + 1

    return i, state.remote_peers[(state.remote_idx % #state.remote_peers) + 1]
  end
end

function _M:iter()
  self.local_tried = 0
  self.remote_tried = 0

  self.local_idx = (self.start_local_idx % #self.local_peers) + 1
  self.remote_idx = (self.start_remote_idx % #self.remote_peers) + 1

  self.start_remote_idx = self.start_remote_idx + 1
  self.start_local_idx = self.start_local_idx + 1

  return next_peer, self, 0
end

return _M
