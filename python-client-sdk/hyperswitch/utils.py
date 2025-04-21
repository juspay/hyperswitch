from typing import Dict, Any

def to_camel_case(snake_str: str) -> str:
    """Convert snake_case string to camelCase."""
    components = snake_str.split('_')
    return components[0] + ''.join(x.title() for x in components[1:])

def serialize_dict(d: Dict[str, Any]) -> Dict[str, Any]:
    """Convert all dictionary keys from snake_case to camelCase recursively."""
    result = {}
    for key, value in d.items():
        if isinstance(value, dict):
            value = serialize_dict(value)
        elif isinstance(value, list):
            value = [serialize_dict(item) if isinstance(item, dict) else item for item in value]
        result[to_camel_case(key)] = value
    return result 