# RoutingKind


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** |  | 
**active_id** | **str** |  | [optional] 
**records** | [**List[RoutingDictionaryRecord]**](RoutingDictionaryRecord.md) |  | 

## Example

```python
from hyperswitch.models.routing_kind import RoutingKind

# TODO update the JSON string below
json = "{}"
# create an instance of RoutingKind from a JSON string
routing_kind_instance = RoutingKind.from_json(json)
# print the JSON string representation of the object
print(RoutingKind.to_json())

# convert the object into a dict
routing_kind_dict = routing_kind_instance.to_dict()
# create an instance of RoutingKind from a dict
routing_kind_from_dict = RoutingKind.from_dict(routing_kind_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


