# RoutingAlgorithm

Routing Algorithm kind

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | [**ProgramConnectorSelection**](ProgramConnectorSelection.md) |  | 

## Example

```python
from hyperswitch.models.routing_algorithm import RoutingAlgorithm

# TODO update the JSON string below
json = "{}"
# create an instance of RoutingAlgorithm from a JSON string
routing_algorithm_instance = RoutingAlgorithm.from_json(json)
# print the JSON string representation of the object
print(RoutingAlgorithm.to_json())

# convert the object into a dict
routing_algorithm_dict = routing_algorithm_instance.to_dict()
# create an instance of RoutingAlgorithm from a dict
routing_algorithm_from_dict = RoutingAlgorithm.from_dict(routing_algorithm_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


