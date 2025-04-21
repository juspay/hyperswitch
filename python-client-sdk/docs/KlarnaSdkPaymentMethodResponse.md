# KlarnaSdkPaymentMethodResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_type** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.klarna_sdk_payment_method_response import KlarnaSdkPaymentMethodResponse

# TODO update the JSON string below
json = "{}"
# create an instance of KlarnaSdkPaymentMethodResponse from a JSON string
klarna_sdk_payment_method_response_instance = KlarnaSdkPaymentMethodResponse.from_json(json)
# print the JSON string representation of the object
print(KlarnaSdkPaymentMethodResponse.to_json())

# convert the object into a dict
klarna_sdk_payment_method_response_dict = klarna_sdk_payment_method_response_instance.to_dict()
# create an instance of KlarnaSdkPaymentMethodResponse from a dict
klarna_sdk_payment_method_response_from_dict = KlarnaSdkPaymentMethodResponse.from_dict(klarna_sdk_payment_method_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


