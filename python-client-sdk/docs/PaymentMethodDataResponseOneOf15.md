# PaymentMethodDataResponseOneOf15


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking** | [**OpenBankingResponse**](OpenBankingResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of15 import PaymentMethodDataResponseOneOf15

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf15 from a JSON string
payment_method_data_response_one_of15_instance = PaymentMethodDataResponseOneOf15.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf15.to_json())

# convert the object into a dict
payment_method_data_response_one_of15_dict = payment_method_data_response_one_of15_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf15 from a dict
payment_method_data_response_one_of15_from_dict = PaymentMethodDataResponseOneOf15.from_dict(payment_method_data_response_one_of15_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


