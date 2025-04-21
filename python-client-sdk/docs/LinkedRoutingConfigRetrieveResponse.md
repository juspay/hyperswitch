# LinkedRoutingConfigRetrieveResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**algorithm** | [**MerchantRoutingAlgorithm**](MerchantRoutingAlgorithm.md) |  | [optional] 

## Example

```python
from hyperswitch.models.linked_routing_config_retrieve_response import LinkedRoutingConfigRetrieveResponse

# TODO update the JSON string below
json = "{}"
# create an instance of LinkedRoutingConfigRetrieveResponse from a JSON string
linked_routing_config_retrieve_response_instance = LinkedRoutingConfigRetrieveResponse.from_json(json)
# print the JSON string representation of the object
print(LinkedRoutingConfigRetrieveResponse.to_json())

# convert the object into a dict
linked_routing_config_retrieve_response_dict = linked_routing_config_retrieve_response_instance.to_dict()
# create an instance of LinkedRoutingConfigRetrieveResponse from a dict
linked_routing_config_retrieve_response_from_dict = LinkedRoutingConfigRetrieveResponse.from_dict(linked_routing_config_retrieve_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


