# RelayResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** | The unique identifier for the Relay | 
**status** | [**RelayStatus**](RelayStatus.md) |  | 
**connector_resource_id** | **str** | The identifier that is associated to a resource at the connector reference to which the relay request is being made | 
**error** | [**RelayError**](RelayError.md) |  | [optional] 
**connector_reference_id** | **str** | The identifier that is associated to a resource at the connector to which the relay request is being made | [optional] 
**connector_id** | **str** | Identifier of the connector ( merchant connector account ) which was chosen to make the payment | 
**profile_id** | **str** | The business profile that is associated with this relay request. | 
**type** | [**RelayType**](RelayType.md) |  | 
**data** | [**RelayData**](RelayData.md) |  | [optional] 

## Example

```python
from hyperswitch.models.relay_response import RelayResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RelayResponse from a JSON string
relay_response_instance = RelayResponse.from_json(json)
# print the JSON string representation of the object
print(RelayResponse.to_json())

# convert the object into a dict
relay_response_dict = relay_response_instance.to_dict()
# create an instance of RelayResponse from a dict
relay_response_from_dict = RelayResponse.from_dict(relay_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


