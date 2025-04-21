# MerchantRoutingAlgorithm

Routing Algorithm specific to merchants

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**profile_id** | **str** |  | 
**name** | **str** |  | 
**description** | **str** |  | 
**algorithm** | [**RoutingAlgorithm**](RoutingAlgorithm.md) |  | 
**created_at** | **int** |  | 
**modified_at** | **int** |  | 
**algorithm_for** | [**TransactionType**](TransactionType.md) |  | 

## Example

```python
from hyperswitch.models.merchant_routing_algorithm import MerchantRoutingAlgorithm

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantRoutingAlgorithm from a JSON string
merchant_routing_algorithm_instance = MerchantRoutingAlgorithm.from_json(json)
# print the JSON string representation of the object
print(MerchantRoutingAlgorithm.to_json())

# convert the object into a dict
merchant_routing_algorithm_dict = merchant_routing_algorithm_instance.to_dict()
# create an instance of MerchantRoutingAlgorithm from a dict
merchant_routing_algorithm_from_dict = MerchantRoutingAlgorithm.from_dict(merchant_routing_algorithm_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


