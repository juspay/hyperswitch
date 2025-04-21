# RoutingDictionaryRecord


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**profile_id** | **str** |  | 
**name** | **str** |  | 
**kind** | [**RoutingAlgorithmKind**](RoutingAlgorithmKind.md) |  | 
**description** | **str** |  | 
**created_at** | **int** |  | 
**modified_at** | **int** |  | 
**algorithm_for** | [**TransactionType**](TransactionType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.routing_dictionary_record import RoutingDictionaryRecord

# TODO update the JSON string below
json = "{}"
# create an instance of RoutingDictionaryRecord from a JSON string
routing_dictionary_record_instance = RoutingDictionaryRecord.from_json(json)
# print the JSON string representation of the object
print(RoutingDictionaryRecord.to_json())

# convert the object into a dict
routing_dictionary_record_dict = routing_dictionary_record_instance.to_dict()
# create an instance of RoutingDictionaryRecord from a dict
routing_dictionary_record_from_dict = RoutingDictionaryRecord.from_dict(routing_dictionary_record_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


