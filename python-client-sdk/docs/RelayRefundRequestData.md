# RelayRefundRequestData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount** | **int** | The amount that is being refunded | 
**currency** | [**Currency**](Currency.md) |  | 
**reason** | **str** | The reason for the refund | [optional] 

## Example

```python
from hyperswitch.models.relay_refund_request_data import RelayRefundRequestData

# TODO update the JSON string below
json = "{}"
# create an instance of RelayRefundRequestData from a JSON string
relay_refund_request_data_instance = RelayRefundRequestData.from_json(json)
# print the JSON string representation of the object
print(RelayRefundRequestData.to_json())

# convert the object into a dict
relay_refund_request_data_dict = relay_refund_request_data_instance.to_dict()
# create an instance of RelayRefundRequestData from a dict
relay_refund_request_data_from_dict = RelayRefundRequestData.from_dict(relay_refund_request_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


