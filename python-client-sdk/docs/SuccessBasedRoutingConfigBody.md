# SuccessBasedRoutingConfigBody


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**min_aggregates_size** | **int** |  | [optional] 
**default_success_rate** | **float** |  | [optional] 
**max_aggregates_size** | **int** |  | [optional] 
**current_block_threshold** | [**CurrentBlockThreshold**](CurrentBlockThreshold.md) |  | [optional] 
**specificity_level** | [**SuccessRateSpecificityLevel**](SuccessRateSpecificityLevel.md) |  | [optional] 

## Example

```python
from hyperswitch.models.success_based_routing_config_body import SuccessBasedRoutingConfigBody

# TODO update the JSON string below
json = "{}"
# create an instance of SuccessBasedRoutingConfigBody from a JSON string
success_based_routing_config_body_instance = SuccessBasedRoutingConfigBody.from_json(json)
# print the JSON string representation of the object
print(SuccessBasedRoutingConfigBody.to_json())

# convert the object into a dict
success_based_routing_config_body_dict = success_based_routing_config_body_instance.to_dict()
# create an instance of SuccessBasedRoutingConfigBody from a dict
success_based_routing_config_body_from_dict = SuccessBasedRoutingConfigBody.from_dict(success_based_routing_config_body_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


