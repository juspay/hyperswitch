# PaymentMethodDataResponseOneOf10


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**upi** | [**UpiResponse**](UpiResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of10 import PaymentMethodDataResponseOneOf10

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf10 from a JSON string
payment_method_data_response_one_of10_instance = PaymentMethodDataResponseOneOf10.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf10.to_json())

# convert the object into a dict
payment_method_data_response_one_of10_dict = payment_method_data_response_one_of10_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf10 from a dict
payment_method_data_response_one_of10_from_dict = PaymentMethodDataResponseOneOf10.from_dict(payment_method_data_response_one_of10_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


