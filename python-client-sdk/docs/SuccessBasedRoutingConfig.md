# SuccessBasedRoutingConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**params** | [**List[DynamicRoutingConfigParams]**](DynamicRoutingConfigParams.md) |  | [optional] 
**config** | [**SuccessBasedRoutingConfigBody**](SuccessBasedRoutingConfigBody.md) |  | [optional] 

## Example

```python
from hyperswitch.models.success_based_routing_config import SuccessBasedRoutingConfig

# TODO update the JSON string below
json = "{}"
# create an instance of SuccessBasedRoutingConfig from a JSON string
success_based_routing_config_instance = SuccessBasedRoutingConfig.from_json(json)
# print the JSON string representation of the object
print(SuccessBasedRoutingConfig.to_json())

# convert the object into a dict
success_based_routing_config_dict = success_based_routing_config_instance.to_dict()
# create an instance of SuccessBasedRoutingConfig from a dict
success_based_routing_config_from_dict = SuccessBasedRoutingConfig.from_dict(success_based_routing_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


