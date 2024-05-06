--- Constant reconnection policy.
-- This policy will allow the cluster module to retry an unhealthy node after
-- a given, constant delay.
-- @module resty.cassandra.policies.reconnection.const
-- @author thibaultcha

local _M = require('resty.cassandra.policies.reconnection').new_policy('constant')

local type = type

--- Create a constant reconnection policy.
-- Instanciates a constant reconnection policy for `resty.cassandra.cluster`
--
-- @usage
-- local Cluster = require "resty.cassandra.cluster"
-- local const_reconn = require "resty.cassandra.policies.reconnection.const"
--
-- local policy = const_reconn.new(60000) -- 1 min
-- local cluster = assert(Cluster.new {
--   reconn_policy = policy
-- })
--
-- @param[type=number] delay Time to wait before trying to reconnect to an
-- unhealthy node, in milliseconds.
-- @treturn table `policy`: A constant reconnection policy.
function _M.new(delay)
  if type(delay) ~= 'number' or delay < 1 then
    error('arg #1 delay must be a positive integer', 2)
  end

  local self = _M.super.new()
  self.delay = delay
  return self
end

function _M:reset()end

function _M:next_delay()
  return self.delay
end

return _M
