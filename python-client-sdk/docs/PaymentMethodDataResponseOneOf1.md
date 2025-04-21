# PaymentMethodDataResponseOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_transfer** | [**BankTransferResponse**](BankTransferResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of1 import PaymentMethodDataResponseOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf1 from a JSON string
payment_method_data_response_one_of1_instance = PaymentMethodDataResponseOneOf1.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf1.to_json())

# convert the object into a dict
payment_method_data_response_one_of1_dict = payment_method_data_response_one_of1_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf1 from a dict
payment_method_data_response_one_of1_from_dict = PaymentMethodDataResponseOneOf1.from_dict(payment_method_data_response_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


