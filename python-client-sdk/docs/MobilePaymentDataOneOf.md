# MobilePaymentDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**direct_carrier_billing** | [**MobilePaymentDataOneOfDirectCarrierBilling**](MobilePaymentDataOneOfDirectCarrierBilling.md) |  | 

## Example

```python
from hyperswitch.models.mobile_payment_data_one_of import MobilePaymentDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of MobilePaymentDataOneOf from a JSON string
mobile_payment_data_one_of_instance = MobilePaymentDataOneOf.from_json(json)
# print the JSON string representation of the object
print(MobilePaymentDataOneOf.to_json())

# convert the object into a dict
mobile_payment_data_one_of_dict = mobile_payment_data_one_of_instance.to_dict()
# create an instance of MobilePaymentDataOneOf from a dict
mobile_payment_data_one_of_from_dict = MobilePaymentDataOneOf.from_dict(mobile_payment_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


