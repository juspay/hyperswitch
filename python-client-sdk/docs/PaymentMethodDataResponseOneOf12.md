# PaymentMethodDataResponseOneOf12


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**gift_card** | [**GiftCardResponse**](GiftCardResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of12 import PaymentMethodDataResponseOneOf12

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf12 from a JSON string
payment_method_data_response_one_of12_instance = PaymentMethodDataResponseOneOf12.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf12.to_json())

# convert the object into a dict
payment_method_data_response_one_of12_dict = payment_method_data_response_one_of12_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf12 from a dict
payment_method_data_response_one_of12_from_dict = PaymentMethodDataResponseOneOf12.from_dict(payment_method_data_response_one_of12_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


