# PaymentMethodDeleteResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method_id** | **str** | The unique identifier of the Payment method | 
**deleted** | **bool** | Whether payment method was deleted or not | 

## Example

```python
from hyperswitch.models.payment_method_delete_response import PaymentMethodDeleteResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDeleteResponse from a JSON string
payment_method_delete_response_instance = PaymentMethodDeleteResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDeleteResponse.to_json())

# convert the object into a dict
payment_method_delete_response_dict = payment_method_delete_response_instance.to_dict()
# create an instance of PaymentMethodDeleteResponse from a dict
payment_method_delete_response_from_dict = PaymentMethodDeleteResponse.from_dict(payment_method_delete_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


