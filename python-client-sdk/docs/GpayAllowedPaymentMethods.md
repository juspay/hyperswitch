# GpayAllowedPaymentMethods


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** | The type of payment method | 
**parameters** | [**GpayAllowedMethodsParameters**](GpayAllowedMethodsParameters.md) |  | 
**tokenization_specification** | [**GpayTokenizationSpecification**](GpayTokenizationSpecification.md) |  | 

## Example

```python
from hyperswitch.models.gpay_allowed_payment_methods import GpayAllowedPaymentMethods

# TODO update the JSON string below
json = "{}"
# create an instance of GpayAllowedPaymentMethods from a JSON string
gpay_allowed_payment_methods_instance = GpayAllowedPaymentMethods.from_json(json)
# print the JSON string representation of the object
print(GpayAllowedPaymentMethods.to_json())

# convert the object into a dict
gpay_allowed_payment_methods_dict = gpay_allowed_payment_methods_instance.to_dict()
# create an instance of GpayAllowedPaymentMethods from a dict
gpay_allowed_payment_methods_from_dict = GpayAllowedPaymentMethods.from_dict(gpay_allowed_payment_methods_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


