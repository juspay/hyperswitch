# RelayRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_resource_id** | **str** | The identifier that is associated to a resource at the connector reference to which the relay request is being made | 
**connector_id** | **str** | Identifier of the connector ( merchant connector account ) which was chosen to make the payment | 
**type** | [**RelayType**](RelayType.md) |  | 
**data** | [**RelayData**](RelayData.md) |  | [optional] 

## Example

```python
from hyperswitch.models.relay_request import RelayRequest

# TODO update the JSON string below
json = "{}"
# create an instance of RelayRequest from a JSON string
relay_request_instance = RelayRequest.from_json(json)
# print the JSON string representation of the object
print(RelayRequest.to_json())

# convert the object into a dict
relay_request_dict = relay_request_instance.to_dict()
# create an instance of RelayRequest from a dict
relay_request_from_dict = RelayRequest.from_dict(relay_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


