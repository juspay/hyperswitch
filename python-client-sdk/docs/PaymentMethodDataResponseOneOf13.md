# PaymentMethodDataResponseOneOf13


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_redirect** | [**CardRedirectResponse**](CardRedirectResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of13 import PaymentMethodDataResponseOneOf13

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf13 from a JSON string
payment_method_data_response_one_of13_instance = PaymentMethodDataResponseOneOf13.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf13.to_json())

# convert the object into a dict
payment_method_data_response_one_of13_dict = payment_method_data_response_one_of13_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf13 from a dict
payment_method_data_response_one_of13_from_dict = PaymentMethodDataResponseOneOf13.from_dict(payment_method_data_response_one_of13_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


