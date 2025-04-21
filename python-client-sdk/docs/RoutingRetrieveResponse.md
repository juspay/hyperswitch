# RoutingRetrieveResponse

Response of the retrieved routing configs for a merchant account

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**algorithm** | [**MerchantRoutingAlgorithm**](MerchantRoutingAlgorithm.md) |  | [optional] 

## Example

```python
from hyperswitch.models.routing_retrieve_response import RoutingRetrieveResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RoutingRetrieveResponse from a JSON string
routing_retrieve_response_instance = RoutingRetrieveResponse.from_json(json)
# print the JSON string representation of the object
print(RoutingRetrieveResponse.to_json())

# convert the object into a dict
routing_retrieve_response_dict = routing_retrieve_response_instance.to_dict()
# create an instance of RoutingRetrieveResponse from a dict
routing_retrieve_response_from_dict = RoutingRetrieveResponse.from_dict(routing_retrieve_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


