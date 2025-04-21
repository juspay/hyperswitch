# ToggleKVResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account | 
**kv_enabled** | **bool** | Status of KV for the specific merchant | 

## Example

```python
from hyperswitch.models.toggle_kv_response import ToggleKVResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ToggleKVResponse from a JSON string
toggle_kv_response_instance = ToggleKVResponse.from_json(json)
# print the JSON string representation of the object
print(ToggleKVResponse.to_json())

# convert the object into a dict
toggle_kv_response_dict = toggle_kv_response_instance.to_dict()
# create an instance of ToggleKVResponse from a dict
toggle_kv_response_from_dict = ToggleKVResponse.from_dict(toggle_kv_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


