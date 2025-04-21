# ResponsePaymentMethodsEnabled


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_types** | [**List[ResponsePaymentMethodTypes]**](ResponsePaymentMethodTypes.md) | The list of payment method types enabled for a connector account | 

## Example

```python
from hyperswitch.models.response_payment_methods_enabled import ResponsePaymentMethodsEnabled

# TODO update the JSON string below
json = "{}"
# create an instance of ResponsePaymentMethodsEnabled from a JSON string
response_payment_methods_enabled_instance = ResponsePaymentMethodsEnabled.from_json(json)
# print the JSON string representation of the object
print(ResponsePaymentMethodsEnabled.to_json())

# convert the object into a dict
response_payment_methods_enabled_dict = response_payment_methods_enabled_instance.to_dict()
# create an instance of ResponsePaymentMethodsEnabled from a dict
response_payment_methods_enabled_from_dict = ResponsePaymentMethodsEnabled.from_dict(response_payment_methods_enabled_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


