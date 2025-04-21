# PaymentChargeType


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe** | [**StripeChargeType**](StripeChargeType.md) |  | 

## Example

```python
from hyperswitch.models.payment_charge_type import PaymentChargeType

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentChargeType from a JSON string
payment_charge_type_instance = PaymentChargeType.from_json(json)
# print the JSON string representation of the object
print(PaymentChargeType.to_json())

# convert the object into a dict
payment_charge_type_dict = payment_charge_type_instance.to_dict()
# create an instance of PaymentChargeType from a dict
payment_charge_type_from_dict = PaymentChargeType.from_dict(payment_charge_type_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


