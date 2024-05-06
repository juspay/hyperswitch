--- Request-aware round robin load balancing policy for OpenResty.
-- This policy will try to reuse the same node for the lifecycle of a given
-- request if possible. It is mostly designed for use in OpenResty
-- environments.
--
-- This ensures that the underlying connection pool to the node is reused as
-- much as possible. If a new node is chosen for every query, there is no
-- guarantee that each selected node will already have a pre-established
-- connection. Since this policy reuses the same node for the lifecycle of a
-- request, the chances of having to open a new connection are much reduced.
-- @module resty.cassandra.policies.lb.req_rr
-- @author thibaultcha

local _rr_lb = require('resty.cassandra.policies.lb').new_policy('req_round_robin')

local past_init

--- Create a request-aware round robin policy.
-- Instanciates a request-aware round robin policy for `resty.cassandra.cluster`.
--
-- @function new
--
-- @usage
-- local Cluster = require "resty.cassandra.cluster"
-- local req_rr = require "resty.cassandra.policies.lb.req_rr"
--
-- local policy = req_rr.new()
-- local cluster = assert(Cluster.new {
--   lb_policy = policy
-- })
--
-- @treturn table `policy`: A request-ware round robin policy.

function _rr_lb:init(peers)
  self.peers = peers
  self.start_idx = -2
end

local function next_peer(state, i)
  if i == #state.peers then
    return nil
  end

  local peer

  if i == 0 and state.initial_cassandra_coordinator then
    peer = state.initial_cassandra_coordinator

  else
    state.idx = state.idx + 1
    peer = state.peers[(state.idx % #state.peers) + 1]
    if peer == state.initial_cassandra_coordinator then
      state.idx = state.idx + 1
      peer = state.peers[(state.idx % #state.peers) + 1]
    end

    if state.ctx then
      state.ctx.cassandra_coordinator = peer
    end
  end

  return i + 1, peer
end

function _rr_lb:iter()
  if past_init or ngx.get_phase() ~= "init" then
    self.ctx = ngx and ngx.ctx
    past_init = true
  end

  if self.ctx then
    self.initial_cassandra_coordinator = self.ctx.cassandra_coordinator
  end

  self.idx = (self.start_idx % #self.peers) + 1
  self.start_idx = self.start_idx + 1

  return next_peer, self, 0
end

return _rr_lb
