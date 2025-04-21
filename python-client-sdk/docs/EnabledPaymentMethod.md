# EnabledPaymentMethod

Object for EnabledPaymentMethod

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_types** | [**List[PaymentMethodType]**](PaymentMethodType.md) | An array of associated payment method types | 

## Example

```python
from hyperswitch.models.enabled_payment_method import EnabledPaymentMethod

# TODO update the JSON string below
json = "{}"
# create an instance of EnabledPaymentMethod from a JSON string
enabled_payment_method_instance = EnabledPaymentMethod.from_json(json)
# print the JSON string representation of the object
print(EnabledPaymentMethod.to_json())

# convert the object into a dict
enabled_payment_method_dict = enabled_payment_method_instance.to_dict()
# create an instance of EnabledPaymentMethod from a dict
enabled_payment_method_from_dict = EnabledPaymentMethod.from_dict(enabled_payment_method_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


