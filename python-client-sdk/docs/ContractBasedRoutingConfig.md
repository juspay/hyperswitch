# ContractBasedRoutingConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**config** | [**ContractBasedRoutingConfigBody**](ContractBasedRoutingConfigBody.md) |  | [optional] 
**label_info** | [**List[LabelInformation]**](LabelInformation.md) |  | [optional] 

## Example

```python
from hyperswitch.models.contract_based_routing_config import ContractBasedRoutingConfig

# TODO update the JSON string below
json = "{}"
# create an instance of ContractBasedRoutingConfig from a JSON string
contract_based_routing_config_instance = ContractBasedRoutingConfig.from_json(json)
# print the JSON string representation of the object
print(ContractBasedRoutingConfig.to_json())

# convert the object into a dict
contract_based_routing_config_dict = contract_based_routing_config_instance.to_dict()
# create an instance of ContractBasedRoutingConfig from a dict
contract_based_routing_config_from_dict = ContractBasedRoutingConfig.from_dict(contract_based_routing_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


