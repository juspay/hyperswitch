# PaymentChargeTypeOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe** | [**StripeChargeType**](StripeChargeType.md) |  | 

## Example

```python
from hyperswitch.models.payment_charge_type_one_of import PaymentChargeTypeOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentChargeTypeOneOf from a JSON string
payment_charge_type_one_of_instance = PaymentChargeTypeOneOf.from_json(json)
# print the JSON string representation of the object
print(PaymentChargeTypeOneOf.to_json())

# convert the object into a dict
payment_charge_type_one_of_dict = payment_charge_type_one_of_instance.to_dict()
# create an instance of PaymentChargeTypeOneOf from a dict
payment_charge_type_one_of_from_dict = PaymentChargeTypeOneOf.from_dict(payment_charge_type_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


