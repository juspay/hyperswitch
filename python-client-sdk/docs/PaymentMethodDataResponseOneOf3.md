# PaymentMethodDataResponseOneOf3


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**pay_later** | [**PaylaterResponse**](PaylaterResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of3 import PaymentMethodDataResponseOneOf3

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf3 from a JSON string
payment_method_data_response_one_of3_instance = PaymentMethodDataResponseOneOf3.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf3.to_json())

# convert the object into a dict
payment_method_data_response_one_of3_dict = payment_method_data_response_one_of3_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf3 from a dict
payment_method_data_response_one_of3_from_dict = PaymentMethodDataResponseOneOf3.from_dict(payment_method_data_response_one_of3_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


