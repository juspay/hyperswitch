# PaymentMethodCreateDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardDetail**](CardDetail.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_create_data_one_of import PaymentMethodCreateDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodCreateDataOneOf from a JSON string
payment_method_create_data_one_of_instance = PaymentMethodCreateDataOneOf.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodCreateDataOneOf.to_json())

# convert the object into a dict
payment_method_create_data_one_of_dict = payment_method_create_data_one_of_instance.to_dict()
# create an instance of PaymentMethodCreateDataOneOf from a dict
payment_method_create_data_one_of_from_dict = PaymentMethodCreateDataOneOf.from_dict(payment_method_create_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


