local _M = {}

function _M.new_policy(name)
  local lb_mt = {
    name = name,
    init = function() error('init() not implemented') end,
    next_peer = function() error('next_peer() not implemented') end,
  }

  lb_mt.__index = lb_mt

  lb_mt.super = {
    new = function()
      return setmetatable({}, lb_mt)
    end
  }

  return setmetatable(lb_mt, {__index = lb_mt.super})
end

return _M
