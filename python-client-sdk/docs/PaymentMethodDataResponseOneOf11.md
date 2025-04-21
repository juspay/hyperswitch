# PaymentMethodDataResponseOneOf11


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**voucher** | [**VoucherResponse**](VoucherResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data_response_one_of11 import PaymentMethodDataResponseOneOf11

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseOneOf11 from a JSON string
payment_method_data_response_one_of11_instance = PaymentMethodDataResponseOneOf11.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseOneOf11.to_json())

# convert the object into a dict
payment_method_data_response_one_of11_dict = payment_method_data_response_one_of11_instance.to_dict()
# create an instance of PaymentMethodDataResponseOneOf11 from a dict
payment_method_data_response_one_of11_from_dict = PaymentMethodDataResponseOneOf11.from_dict(payment_method_data_response_one_of11_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


