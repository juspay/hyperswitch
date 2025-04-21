# PaymentMethodDataResponseOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**wallet** | [**WalletResponse**](WalletResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of2 import PaymentMethodDataResponseOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf2 from a JSON string
payment_method_data_response_one_of2_instance = PaymentMethodDataResponseOneOf2.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf2.to_json())

# convert the object into a dict
payment_method_data_response_one_of2_dict = payment_method_data_response_one_of2_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf2 from a dict
payment_method_data_response_one_of2_from_dict = PaymentMethodDataResponseOneOf2.from_dict(payment_method_data_response_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


