# BlocklistResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**fingerprint_id** | **str** |  | 
**data_kind** | [**BlocklistDataKind**](BlocklistDataKind.md) |  | 
**created_at** | **datetime** |  | 

## Example

```python
from hyperswitch.models.blocklist_response import BlocklistResponse

# TODO update the JSON string below
json = "{}"
# create an instance of BlocklistResponse from a JSON string
blocklist_response_instance = BlocklistResponse.from_json(json)
# print the JSON string representation of the object
print(BlocklistResponse.to_json())

# convert the object into a dict
blocklist_response_dict = blocklist_response_instance.to_dict()
# create an instance of BlocklistResponse from a dict
blocklist_response_from_dict = BlocklistResponse.from_dict(blocklist_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


