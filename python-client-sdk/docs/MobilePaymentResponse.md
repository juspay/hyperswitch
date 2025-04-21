# MobilePaymentResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**direct_carrier_billing** | [**MobilePaymentDataOneOfDirectCarrierBilling**](MobilePaymentDataOneOfDirectCarrierBilling.md) |  | 

## Example

```python
from hyperswitch.models.mobile_payment_response import MobilePaymentResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MobilePaymentResponse from a JSON string
mobile_payment_response_instance = MobilePaymentResponse.from_json(json)
# print the JSON string representation of the object
print(MobilePaymentResponse.to_json())

# convert the object into a dict
mobile_payment_response_dict = mobile_payment_response_instance.to_dict()
# create an instance of MobilePaymentResponse from a dict
mobile_payment_response_from_dict = MobilePaymentResponse.from_dict(mobile_payment_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


