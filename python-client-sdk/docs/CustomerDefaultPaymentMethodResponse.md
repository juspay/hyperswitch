# CustomerDefaultPaymentMethodResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**default_payment_method_id** | **str** | The unique identifier of the Payment method | [optional] 
**customer_id** | **str** | The unique identifier of the customer. | 
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.customer_default_payment_method_response import CustomerDefaultPaymentMethodResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerDefaultPaymentMethodResponse from a JSON string
customer_default_payment_method_response_instance = CustomerDefaultPaymentMethodResponse.from_json(json)
# print the JSON string representation of the object
print(CustomerDefaultPaymentMethodResponse.to_json())

# convert the object into a dict
customer_default_payment_method_response_dict = customer_default_payment_method_response_instance.to_dict()
# create an instance of CustomerDefaultPaymentMethodResponse from a dict
customer_default_payment_method_response_from_dict = CustomerDefaultPaymentMethodResponse.from_dict(customer_default_payment_method_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


