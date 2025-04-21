# ApplepayPaymentMethod


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**display_name** | **str** | The name to be displayed on Apple Pay button | 
**network** | **str** | The network of the Apple pay payment method | 
**type** | **str** | The type of the payment method | 

## Example

```python
from hyperswitch.models.applepay_payment_method import ApplepayPaymentMethod

# TODO update the JSON string below
json = "{}"
# create an instance of ApplepayPaymentMethod from a JSON string
applepay_payment_method_instance = ApplepayPaymentMethod.from_json(json)
# print the JSON string representation of the object
print(ApplepayPaymentMethod.to_json())

# convert the object into a dict
applepay_payment_method_dict = applepay_payment_method_instance.to_dict()
# create an instance of ApplepayPaymentMethod from a dict
applepay_payment_method_from_dict = ApplepayPaymentMethod.from_dict(applepay_payment_method_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


