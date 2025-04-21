# PaymentMethodDataResponseOneOf6


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_debit** | [**BankDebitResponse**](BankDebitResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of6 import PaymentMethodDataResponseOneOf6

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf6 from a JSON string
payment_method_data_response_one_of6_instance = PaymentMethodDataResponseOneOf6.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf6.to_json())

# convert the object into a dict
payment_method_data_response_one_of6_dict = payment_method_data_response_one_of6_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf6 from a dict
payment_method_data_response_one_of6_from_dict = PaymentMethodDataResponseOneOf6.from_dict(payment_method_data_response_one_of6_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


