# RoutingConfigRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** |  | [optional] 
**description** | **str** |  | [optional] 
**algorithm** | [**RoutingAlgorithm**](RoutingAlgorithm.md) |  | [optional] 
**profile_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.routing_config_request import RoutingConfigRequest

# TODO update the JSON string below
json = "{}"
# create an instance of RoutingConfigRequest from a JSON string
routing_config_request_instance = RoutingConfigRequest.from_json(json)
# print the JSON string representation of the object
print(RoutingConfigRequest.to_json())

# convert the object into a dict
routing_config_request_dict = routing_config_request_instance.to_dict()
# create an instance of RoutingConfigRequest from a dict
routing_config_request_from_dict = RoutingConfigRequest.from_dict(routing_config_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


