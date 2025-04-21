# CustomerDeleteResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_id** | **str** | The identifier for the customer object | 
**customer_deleted** | **bool** | Whether customer was deleted or not | 
**address_deleted** | **bool** | Whether address was deleted or not | 
**payment_methods_deleted** | **bool** | Whether payment methods deleted or not | 

## Example

```python
from hyperswitch.models.customer_delete_response import CustomerDeleteResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerDeleteResponse from a JSON string
customer_delete_response_instance = CustomerDeleteResponse.from_json(json)
# print the JSON string representation of the object
print(CustomerDeleteResponse.to_json())

# convert the object into a dict
customer_delete_response_dict = customer_delete_response_instance.to_dict()
# create an instance of CustomerDeleteResponse from a dict
customer_delete_response_from_dict = CustomerDeleteResponse.from_dict(customer_delete_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


