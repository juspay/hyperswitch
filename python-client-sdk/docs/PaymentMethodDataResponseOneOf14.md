# PaymentMethodDataResponseOneOf14


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_token** | [**CardTokenResponse**](CardTokenResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of14 import PaymentMethodDataResponseOneOf14

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf14 from a JSON string
payment_method_data_response_one_of14_instance = PaymentMethodDataResponseOneOf14.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf14.to_json())

# convert the object into a dict
payment_method_data_response_one_of14_dict = payment_method_data_response_one_of14_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf14 from a dict
payment_method_data_response_one_of14_from_dict = PaymentMethodDataResponseOneOf14.from_dict(payment_method_data_response_one_of14_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


