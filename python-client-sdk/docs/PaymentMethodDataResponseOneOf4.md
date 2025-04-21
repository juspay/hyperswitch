# PaymentMethodDataResponseOneOf4


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_redirect** | [**BankRedirectResponse**](BankRedirectResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of4 import PaymentMethodDataResponseOneOf4

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf4 from a JSON string
payment_method_data_response_one_of4_instance = PaymentMethodDataResponseOneOf4.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf4.to_json())

# convert the object into a dict
payment_method_data_response_one_of4_dict = payment_method_data_response_one_of4_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf4 from a dict
payment_method_data_response_one_of4_from_dict = PaymentMethodDataResponseOneOf4.from_dict(payment_method_data_response_one_of4_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


