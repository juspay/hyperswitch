# BlocklistRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | **str** |  | 

## Example

```python
from hyperswitch.models.blocklist_request import BlocklistRequest

# TODO update the JSON string below
json = "{}"
# create an instance of BlocklistRequest from a JSON string
blocklist_request_instance = BlocklistRequest.from_json(json)
# print the JSON string representation of the object
print(BlocklistRequest.to_json())

# convert the object into a dict
blocklist_request_dict = blocklist_request_instance.to_dict()
# create an instance of BlocklistRequest from a dict
blocklist_request_from_dict = BlocklistRequest.from_dict(blocklist_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


