# PaymentMethodCreateData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardDetail**](CardDetail.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_create_data import PaymentMethodCreateData

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodCreateData from a JSON string
payment_method_create_data_instance = PaymentMethodCreateData.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodCreateData.to_json())

# convert the object into a dict
payment_method_create_data_dict = payment_method_create_data_instance.to_dict()
# create an instance of PaymentMethodCreateData from a dict
payment_method_create_data_from_dict = PaymentMethodCreateData.from_dict(payment_method_create_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


