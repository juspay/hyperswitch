# ProfileDefaultRoutingConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**profile_id** | **str** |  | 
**connectors** | [**List[RoutableConnectorChoice]**](RoutableConnectorChoice.md) |  | 

## Example

```python
from hyperswitch.models.profile_default_routing_config import ProfileDefaultRoutingConfig

# TODO update the JSON string below
json = "{}"
# create an instance of ProfileDefaultRoutingConfig from a JSON string
profile_default_routing_config_instance = ProfileDefaultRoutingConfig.from_json(json)
# print the JSON string representation of the object
print(ProfileDefaultRoutingConfig.to_json())

# convert the object into a dict
profile_default_routing_config_dict = profile_default_routing_config_instance.to_dict()
# create an instance of ProfileDefaultRoutingConfig from a dict
profile_default_routing_config_from_dict = ProfileDefaultRoutingConfig.from_dict(profile_default_routing_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


