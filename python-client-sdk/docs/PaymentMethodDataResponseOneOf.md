# PaymentMethodDataResponseOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardResponse**](CardResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of import PaymentMethodDataResponseOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf from a JSON string
payment_method_data_response_one_of_instance = PaymentMethodDataResponseOneOf.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf.to_json())

# convert the object into a dict
payment_method_data_response_one_of_dict = payment_method_data_response_one_of_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf from a dict
payment_method_data_response_one_of_from_dict = PaymentMethodDataResponseOneOf.from_dict(payment_method_data_response_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


