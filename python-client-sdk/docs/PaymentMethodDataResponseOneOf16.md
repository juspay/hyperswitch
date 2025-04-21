# PaymentMethodDataResponseOneOf16


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**mobile_payment** | [**MobilePaymentResponse**](MobilePaymentResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of16 import PaymentMethodDataResponseOneOf16

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf16 from a JSON string
payment_method_data_response_one_of16_instance = PaymentMethodDataResponseOneOf16.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf16.to_json())

# convert the object into a dict
payment_method_data_response_one_of16_dict = payment_method_data_response_one_of16_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf16 from a dict
payment_method_data_response_one_of16_from_dict = PaymentMethodDataResponseOneOf16.from_dict(payment_method_data_response_one_of16_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


