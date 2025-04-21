# PaymentMethodsEnabled

Details of all the payment methods enabled for the connector for the given merchant account

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_types** | [**List[RequestPaymentMethodTypes]**](RequestPaymentMethodTypes.md) | Subtype of payment method | [optional] 

## Example

```python
from hyperswitch.models.payment_methods_enabled import PaymentMethodsEnabled

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodsEnabled from a JSON string
payment_methods_enabled_instance = PaymentMethodsEnabled.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodsEnabled.to_json())

# convert the object into a dict
payment_methods_enabled_dict = payment_methods_enabled_instance.to_dict()
# create an instance of PaymentMethodsEnabled from a dict
payment_methods_enabled_from_dict = PaymentMethodsEnabled.from_dict(payment_methods_enabled_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


