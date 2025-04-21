# MobilePaymentData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**direct_carrier_billing** | [**MobilePaymentDataOneOfDirectCarrierBilling**](MobilePaymentDataOneOfDirectCarrierBilling.md) |  | 

## Example

```python
from hyperswitch.models.mobile_payment_data import MobilePaymentData

# TODO update the JSON string below
json = "{}"
# create an instance of MobilePaymentData from a JSON string
mobile_payment_data_instance = MobilePaymentData.from_json(json)
# print the JSON string representation of the object
print(MobilePaymentData.to_json())

# convert the object into a dict
mobile_payment_data_dict = mobile_payment_data_instance.to_dict()
# create an instance of MobilePaymentData from a dict
mobile_payment_data_from_dict = MobilePaymentData.from_dict(mobile_payment_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


