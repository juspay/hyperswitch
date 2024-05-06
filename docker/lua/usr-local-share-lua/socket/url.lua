-----------------------------------------------------------------------------
-- URI parsing, composition and relative URL resolution
-- LuaSocket toolkit.
-- Author: Diego Nehab
-----------------------------------------------------------------------------

-----------------------------------------------------------------------------
-- Declare module
-----------------------------------------------------------------------------
local string = require("string")
local base = _G
local table = require("table")
local socket = require("socket")

socket.url = {}
local _M = socket.url

-----------------------------------------------------------------------------
-- Module version
-----------------------------------------------------------------------------
_M._VERSION = "URL 1.0.3"

-----------------------------------------------------------------------------
-- Encodes a string into its escaped hexadecimal representation
-- Input
--   s: binary string to be encoded
-- Returns
--   escaped representation of string binary
-----------------------------------------------------------------------------
function _M.escape(s)
    return (string.gsub(s, "([^A-Za-z0-9_])", function(c)
        return string.format("%%%02x", string.byte(c))
    end))
end

-----------------------------------------------------------------------------
-- Protects a path segment, to prevent it from interfering with the
-- url parsing.
-- Input
--   s: binary string to be encoded
-- Returns
--   escaped representation of string binary
-----------------------------------------------------------------------------
local function make_set(t)
    local s = {}
    for i,v in base.ipairs(t) do
        s[t[i]] = 1
    end
    return s
end

-- these are allowed within a path segment, along with alphanum
-- other characters must be escaped
local segment_set = make_set {
    "-", "_", ".", "!", "~", "*", "'", "(",
    ")", ":", "@", "&", "=", "+", "$", ",",
}

local function protect_segment(s)
    return string.gsub(s, "([^A-Za-z0-9_])", function (c)
        if segment_set[c] then return c
        else return string.format("%%%02X", string.byte(c)) end
    end)
end

-----------------------------------------------------------------------------
-- Unencodes a escaped hexadecimal string into its binary representation
-- Input
--   s: escaped hexadecimal string to be unencoded
-- Returns
--   unescaped binary representation of escaped hexadecimal  binary
-----------------------------------------------------------------------------
function _M.unescape(s)
    return (string.gsub(s, "%%(%x%x)", function(hex)
        return string.char(base.tonumber(hex, 16))
    end))
end

-----------------------------------------------------------------------------
-- Removes '..' and '.' components appropriately from a path.
-- Input
--   path
-- Returns
--   dot-normalized path
local function remove_dot_components(path)
    local marker = string.char(1)
    repeat
        local was = path
        path = path:gsub('//', '/'..marker..'/', 1)
    until path == was
    repeat
        local was = path
        path = path:gsub('/%./', '/', 1)
    until path == was
    repeat
        local was = path
        path = path:gsub('[^/]+/%.%./([^/]+)', '%1', 1)
    until path == was
    path = path:gsub('[^/]+/%.%./*$', '')
    path = path:gsub('/%.%.$', '/')
    path = path:gsub('/%.$', '/')
    path = path:gsub('^/%.%./', '/')
    path = path:gsub(marker, '')
    return path
end

-----------------------------------------------------------------------------
-- Builds a path from a base path and a relative path
-- Input
--   base_path
--   relative_path
-- Returns
--   corresponding absolute path
-----------------------------------------------------------------------------
local function absolute_path(base_path, relative_path)
    if string.sub(relative_path, 1, 1) == "/" then
      return remove_dot_components(relative_path) end
    base_path = base_path:gsub("[^/]*$", "")
    if not base_path:find'/$' then base_path = base_path .. '/' end
    local path = base_path .. relative_path
    path = remove_dot_components(path)
    return path
end

-----------------------------------------------------------------------------
-- Parses a url and returns a table with all its parts according to RFC 2396
-- The following grammar describes the names given to the URL parts
-- <url> ::= <scheme>://<authority>/<path>;<params>?<query>#<fragment>
-- <authority> ::= <userinfo>@<host>:<port>
-- <userinfo> ::= <user>[:<password>]
-- <path> :: = {<segment>/}<segment>
-- Input
--   url: uniform resource locator of request
--   default: table with default values for each field
-- Returns
--   table with the following fields, where RFC naming conventions have
--   been preserved:
--     scheme, authority, userinfo, user, password, host, port,
--     path, params, query, fragment
-- Obs:
--   the leading '/' in {/<path>} is considered part of <path>
-----------------------------------------------------------------------------
function _M.parse(url, default)
    -- initialize default parameters
    local parsed = {}
    for i,v in base.pairs(default or parsed) do parsed[i] = v end
    -- empty url is parsed to nil
    if not url or url == "" then return nil, "invalid url" end
    -- remove whitespace
    -- url = string.gsub(url, "%s", "")
    -- get scheme
    url = string.gsub(url, "^([%w][%w%+%-%.]*)%:",
        function(s) parsed.scheme = s; return "" end)
    -- get authority
    url = string.gsub(url, "^//([^/]*)", function(n)
        parsed.authority = n
        return ""
    end)
    -- get fragment
    url = string.gsub(url, "#(.*)$", function(f)
        parsed.fragment = f
        return ""
    end)
    -- get query string
    url = string.gsub(url, "%?(.*)", function(q)
        parsed.query = q
        return ""
    end)
    -- get params
    url = string.gsub(url, "%;(.*)", function(p)
        parsed.params = p
        return ""
    end)
    -- path is whatever was left
    if url ~= "" then parsed.path = url end
    local authority = parsed.authority
    if not authority then return parsed end
    authority = string.gsub(authority,"^([^@]*)@",
        function(u) parsed.userinfo = u; return "" end)
    authority = string.gsub(authority, ":([^:%]]*)$",
        function(p) parsed.port = p; return "" end)
    if authority ~= "" then
        -- IPv6?
        parsed.host = string.match(authority, "^%[(.+)%]$") or authority
    end
    local userinfo = parsed.userinfo
    if not userinfo then return parsed end
    userinfo = string.gsub(userinfo, ":([^:]*)$",
        function(p) parsed.password = p; return "" end)
    parsed.user = userinfo
    return parsed
end

-----------------------------------------------------------------------------
-- Rebuilds a parsed URL from its components.
-- Components are protected if any reserved or unallowed characters are found
-- Input
--   parsed: parsed URL, as returned by parse
-- Returns
--   a stringing with the corresponding URL
-----------------------------------------------------------------------------
function _M.build(parsed)
    --local ppath = _M.parse_path(parsed.path or "")
    --local url = _M.build_path(ppath)
    local url = parsed.path or ""
    if parsed.params then url = url .. ";" .. parsed.params end
    if parsed.query then url = url .. "?" .. parsed.query end
    local authority = parsed.authority
    if parsed.host then
        authority = parsed.host
        if string.find(authority, ":") then -- IPv6?
            authority = "[" .. authority .. "]"
        end
        if parsed.port then authority = authority .. ":" .. base.tostring(parsed.port) end
        local userinfo = parsed.userinfo
        if parsed.user then
            userinfo = parsed.user
            if parsed.password then
                userinfo = userinfo .. ":" .. parsed.password
            end
        end
        if userinfo then authority = userinfo .. "@" .. authority end
    end
    if authority then url = "//" .. authority .. url end
    if parsed.scheme then url = parsed.scheme .. ":" .. url end
    if parsed.fragment then url = url .. "#" .. parsed.fragment end
    -- url = string.gsub(url, "%s", "")
    return url
end

-----------------------------------------------------------------------------
-- Builds a absolute URL from a base and a relative URL according to RFC 2396
-- Input
--   base_url
--   relative_url
-- Returns
--   corresponding absolute url
-----------------------------------------------------------------------------
function _M.absolute(base_url, relative_url)
    local base_parsed
    if base.type(base_url) == "table" then
        base_parsed = base_url
        base_url = _M.build(base_parsed)
    else
        base_parsed = _M.parse(base_url)
    end
    local result
    local relative_parsed = _M.parse(relative_url)
    if not base_parsed then
        result = relative_url
    elseif not relative_parsed then
        result = base_url
    elseif relative_parsed.scheme then
        result = relative_url
    else
        relative_parsed.scheme = base_parsed.scheme
        if not relative_parsed.authority then
            relative_parsed.authority = base_parsed.authority
            if not relative_parsed.path then
                relative_parsed.path = base_parsed.path
                if not relative_parsed.params then
                    relative_parsed.params = base_parsed.params
                    if not relative_parsed.query then
                        relative_parsed.query = base_parsed.query
                    end
                end
            else
                relative_parsed.path = absolute_path(base_parsed.path or "",
                    relative_parsed.path)
            end
        end
        result = _M.build(relative_parsed)
    end
    return remove_dot_components(result)
end

-----------------------------------------------------------------------------
-- Breaks a path into its segments, unescaping the segments
-- Input
--   path
-- Returns
--   segment: a table with one entry per segment
-----------------------------------------------------------------------------
function _M.parse_path(path)
    local parsed = {}
    path = path or ""
    --path = string.gsub(path, "%s", "")
    string.gsub(path, "([^/]+)", function (s) table.insert(parsed, s) end)
    for i = 1, #parsed do
        parsed[i] = _M.unescape(parsed[i])
    end
    if string.sub(path, 1, 1) == "/" then parsed.is_absolute = 1 end
    if string.sub(path, -1, -1) == "/" then parsed.is_directory = 1 end
    return parsed
end

-----------------------------------------------------------------------------
-- Builds a path component from its segments, escaping protected characters.
-- Input
--   parsed: path segments
--   unsafe: if true, segments are not protected before path is built
-- Returns
--   path: corresponding path stringing
-----------------------------------------------------------------------------
function _M.build_path(parsed, unsafe)
    local path = ""
    local n = #parsed
    if unsafe then
        for i = 1, n-1 do
            path = path .. parsed[i]
            path = path .. "/"
        end
        if n > 0 then
            path = path .. parsed[n]
            if parsed.is_directory then path = path .. "/" end
        end
    else
        for i = 1, n-1 do
            path = path .. protect_segment(parsed[i])
            path = path .. "/"
        end
        if n > 0 then
            path = path .. protect_segment(parsed[n])
            if parsed.is_directory then path = path .. "/" end
        end
    end
    if parsed.is_absolute then path = "/" .. path end
    return path
end

return _M
