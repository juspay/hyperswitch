# ToggleBlocklistResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**blocklist_guard_status** | **str** |  | 

## Example

```python
from hyperswitch.models.toggle_blocklist_response import ToggleBlocklistResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ToggleBlocklistResponse from a JSON string
toggle_blocklist_response_instance = ToggleBlocklistResponse.from_json(json)
# print the JSON string representation of the object
print(ToggleBlocklistResponse.to_json())

# convert the object into a dict
toggle_blocklist_response_dict = toggle_blocklist_response_instance.to_dict()
# create an instance of ToggleBlocklistResponse from a dict
toggle_blocklist_response_from_dict = ToggleBlocklistResponse.from_dict(toggle_blocklist_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


