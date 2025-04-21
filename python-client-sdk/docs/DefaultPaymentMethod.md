# DefaultPaymentMethod


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_id** | **str** |  | 
**payment_method_id** | **str** |  | 

## Example

```python
from hyperswitch.models.default_payment_method import DefaultPaymentMethod

# TODO update the JSON string below
json = "{}"
# create an instance of DefaultPaymentMethod from a JSON string
default_payment_method_instance = DefaultPaymentMethod.from_json(json)
# print the JSON string representation of the object
print(DefaultPaymentMethod.to_json())

# convert the object into a dict
default_payment_method_dict = default_payment_method_instance.to_dict()
# create an instance of DefaultPaymentMethod from a dict
default_payment_method_from_dict = DefaultPaymentMethod.from_dict(default_payment_method_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


