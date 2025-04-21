# ContractBasedRoutingConfigBody


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**constants** | **List[float]** |  | [optional] 
**time_scale** | [**ContractBasedTimeScale**](ContractBasedTimeScale.md) |  | [optional] 

## Example

```python
from hyperswitch.models.contract_based_routing_config_body import ContractBasedRoutingConfigBody

# TODO update the JSON string below
json = "{}"
# create an instance of ContractBasedRoutingConfigBody from a JSON string
contract_based_routing_config_body_instance = ContractBasedRoutingConfigBody.from_json(json)
# print the JSON string representation of the object
print(ContractBasedRoutingConfigBody.to_json())

# convert the object into a dict
contract_based_routing_config_body_dict = contract_based_routing_config_body_instance.to_dict()
# create an instance of ContractBasedRoutingConfigBody from a dict
contract_based_routing_config_body_from_dict = ContractBasedRoutingConfigBody.from_dict(contract_based_routing_config_body_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


