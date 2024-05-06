local _M = {}

function _M.new_policy(name)
  local retry_mt = {
    name = name,
    on_unavailable = function() error('on_unavailable() not implemented') end,
    on_read_timeout = function() error('on_read_timeout() not implemented') end,
    on_write_timeout = function() error('on_write_timeout() not implemented') end,
  }

  retry_mt.__index = retry_mt

  retry_mt.super = {
    new = function()
      return setmetatable({}, retry_mt)
    end
  }

  return setmetatable(retry_mt, {__index = retry_mt.super})
end

return _M
