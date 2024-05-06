-----------------------------------------------------------------------------
-- MIME support for the Lua language.
-- Author: Diego Nehab
-- Conforming to RFCs 2045-2049
-----------------------------------------------------------------------------

-----------------------------------------------------------------------------
-- Declare module and import dependencies
-----------------------------------------------------------------------------
local base = _G
local ltn12 = require("ltn12")
local mime = require("mime.core")
local _M = mime

-- encode, decode and wrap algorithm tables
local encodet, decodet, wrapt = {},{},{}

_M.encodet = encodet
_M.decodet = decodet
_M.wrapt   = wrapt

-- creates a function that chooses a filter by name from a given table
local function choose(table)
    return function(name, opt1, opt2)
        if base.type(name) ~= "string" then
            name, opt1, opt2 = "default", name, opt1
        end
        local f = table[name or "nil"]
        if not f then
            base.error("unknown key (" .. base.tostring(name) .. ")", 3)
        else return f(opt1, opt2) end
    end
end

-- define the encoding filters
encodet['base64'] = function()
    return ltn12.filter.cycle(_M.b64, "")
end

encodet['quoted-printable'] = function(mode)
    return ltn12.filter.cycle(_M.qp, "",
        (mode == "binary") and "=0D=0A" or "\r\n")
end

-- define the decoding filters
decodet['base64'] = function()
    return ltn12.filter.cycle(_M.unb64, "")
end

decodet['quoted-printable'] = function()
    return ltn12.filter.cycle(_M.unqp, "")
end

-- define the line-wrap filters
wrapt['text'] = function(length)
    length = length or 76
    return ltn12.filter.cycle(_M.wrp, length, length)
end
wrapt['base64'] = wrapt['text']
wrapt['default'] = wrapt['text']

wrapt['quoted-printable'] = function()
    return ltn12.filter.cycle(_M.qpwrp, 76, 76)
end

-- function that choose the encoding, decoding or wrap algorithm
_M.encode = choose(encodet)
_M.decode = choose(decodet)
_M.wrap = choose(wrapt)

-- define the end-of-line normalization filter
function _M.normalize(marker)
    return ltn12.filter.cycle(_M.eol, 0, marker)
end

-- high level stuffing filter
function _M.stuff()
    return ltn12.filter.cycle(_M.dot, 2)
end

return _M
