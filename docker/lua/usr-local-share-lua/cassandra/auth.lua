local setmetatable = setmetatable
local type = type
local fmt = string.format

local plain_text = {}
plain_text.__index = plain_text

function plain_text.new(username, password)
  if type(username) ~= 'string' then
    error('arg #1 must be a string (username)', 3)
  elseif type(password) ~= 'string' then
    error('arg #2 must be a string (password)', 3)
  end

  return setmetatable({
    username = username,
    password = password
  }, plain_text)
end

function plain_text:initial_response()
  return fmt('\0%s\0%s', self.username, self.password)
end

return {
  plain_text = setmetatable(plain_text, {
    __call = function(_, ...)
      return plain_text.new(...)
    end
  })
}
