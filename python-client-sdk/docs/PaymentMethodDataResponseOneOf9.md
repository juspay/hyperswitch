# PaymentMethodDataResponseOneOf9


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**real_time_payment** | [**RealTimePaymentDataResponse**](RealTimePaymentDataResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of9 import PaymentMethodDataResponseOneOf9

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf9 from a JSON string
payment_method_data_response_one_of9_instance = PaymentMethodDataResponseOneOf9.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf9.to_json())

# convert the object into a dict
payment_method_data_response_one_of9_dict = payment_method_data_response_one_of9_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf9 from a dict
payment_method_data_response_one_of9_from_dict = PaymentMethodDataResponseOneOf9.from_dict(payment_method_data_response_one_of9_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


