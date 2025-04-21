# CustomerPaymentMethodsListResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_payment_methods** | [**List[CustomerPaymentMethod]**](CustomerPaymentMethod.md) | List of payment methods for customer | 
**is_guest_customer** | **bool** | Returns whether a customer id is not tied to a payment intent (only when the request is made against a client secret) | [optional] 

## Example

```python
from hyperswitch.models.customer_payment_methods_list_response import CustomerPaymentMethodsListResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerPaymentMethodsListResponse from a JSON string
customer_payment_methods_list_response_instance = CustomerPaymentMethodsListResponse.from_json(json)
# print the JSON string representation of the object
print(CustomerPaymentMethodsListResponse.to_json())

# convert the object into a dict
customer_payment_methods_list_response_dict = customer_payment_methods_list_response_instance.to_dict()
# create an instance of CustomerPaymentMethodsListResponse from a dict
customer_payment_methods_list_response_from_dict = CustomerPaymentMethodsListResponse.from_dict(customer_payment_methods_list_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


