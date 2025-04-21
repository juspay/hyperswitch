# PaymentsCancelRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**cancellation_reason** | **str** | The reason for the payment cancel | [optional] 
**merchant_connector_details** | [**MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payments_cancel_request import PaymentsCancelRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsCancelRequest from a JSON string
payments_cancel_request_instance = PaymentsCancelRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsCancelRequest.to_json())

# convert the object into a dict
payments_cancel_request_dict = payments_cancel_request_instance.to_dict()
# create an instance of PaymentsCancelRequest from a dict
payments_cancel_request_from_dict = PaymentsCancelRequest.from_dict(payments_cancel_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


