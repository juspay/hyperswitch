
local results = {}
for i = 1, #KEYS do
    results[i] = redis.call("INCRBY", KEYS[i], ARGV[i])
end
return results