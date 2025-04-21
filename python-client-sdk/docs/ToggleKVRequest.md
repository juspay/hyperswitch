# ToggleKVRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**kv_enabled** | **bool** | Status of KV for the specific merchant | 

## Example

```python
from hyperswitch.models.toggle_kv_request import ToggleKVRequest

# TODO update the JSON string below
json = "{}"
# create an instance of ToggleKVRequest from a JSON string
toggle_kv_request_instance = ToggleKVRequest.from_json(json)
# print the JSON string representation of the object
print(ToggleKVRequest.to_json())

# convert the object into a dict
toggle_kv_request_dict = toggle_kv_request_instance.to_dict()
# create an instance of ToggleKVRequest from a dict
toggle_kv_request_from_dict = ToggleKVRequest.from_dict(toggle_kv_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


