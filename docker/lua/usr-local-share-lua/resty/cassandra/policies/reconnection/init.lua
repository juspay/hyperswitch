local _M = {}

function _M.new_policy(name)
  local reconn_mt = {
    name = name,
    reset = function() error('reset() not implemented') end,
    next_delay = function() error('next_delay() not implemented') end,
  }

  reconn_mt.__index = reconn_mt

  reconn_mt.super = {
    new = function()
      return setmetatable({}, reconn_mt)
    end
  }

  return setmetatable(reconn_mt, {__index = reconn_mt.super})
end

return _M
