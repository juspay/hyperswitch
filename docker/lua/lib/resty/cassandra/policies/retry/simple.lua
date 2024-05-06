--- Simple retry policy.
-- This policy will retry requests that failed because of
-- UNAVAILABLE_EXCEPTION, READ_TIMEOUT or WRITE_TIMEOUT server errors up to a
-- given number of time before failing and returning an error.
-- @module resty.cassandra.policies.retry.simple
-- @author thibaultcha

local _M = require('resty.cassandra.policies.retry').new_policy('simple')

local type = type

--- Create a simple retry policy.
-- Instanciates a simple retry policy for
-- `resty.cassandra.cluster`.
--
-- @usage
-- local Cluster = require "resty.cassandra.cluster"
-- local simple_retry = require "resty.cassandra.policies.retry.simple"
--
-- local policy = simple_retry.new(3)
-- local cluster = assert(Cluster.new {
--   retry_policy = policy
-- })
--
-- @param[type=number] max_retries Maximum number of retries for a query
-- before aborting and reporting the error.
-- @treturn table `policy`: A simple retry policy.
function _M.new(max_retries)
  if type(max_retries) ~= 'number' or max_retries < 1 then
    error('arg #1 max_retries must be a positive integer', 2)
  end

  local self = _M.super.new()
  self.max_retries = max_retries
  return self
end

function _M:on_unavailable(request)
  return false
end

function _M:on_read_timeout(request)
  return request.retries < self.max_retries
end

function _M:on_write_timeout(request)
  return request.retries < self.max_retries
end

return _M
