# RoutingDictionary


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** |  | 
**active_id** | **str** |  | [optional] 
**records** | [**List[RoutingDictionaryRecord]**](RoutingDictionaryRecord.md) |  | 

## Example

```python
from hyperswitch.models.routing_dictionary import RoutingDictionary

# TODO update the JSON string below
json = "{}"
# create an instance of RoutingDictionary from a JSON string
routing_dictionary_instance = RoutingDictionary.from_json(json)
# print the JSON string representation of the object
print(RoutingDictionary.to_json())

# convert the object into a dict
routing_dictionary_dict = routing_dictionary_instance.to_dict()
# create an instance of RoutingDictionary from a dict
routing_dictionary_from_dict = RoutingDictionary.from_dict(routing_dictionary_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


