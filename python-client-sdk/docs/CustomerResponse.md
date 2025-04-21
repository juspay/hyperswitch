# CustomerResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_id** | **str** | The identifier for the customer object | 
**name** | **str** | The customer&#39;s name | [optional] 
**email** | **str** | The customer&#39;s email address | [optional] 
**phone** | **str** | The customer&#39;s phone number | [optional] 
**phone_country_code** | **str** | The country code for the customer phone number | [optional] 
**description** | **str** | An arbitrary string that you can attach to a customer object. | [optional] 
**address** | [**AddressDetails**](AddressDetails.md) |  | [optional] 
**created_at** | **datetime** | A timestamp (ISO 8601 code) that determines when the customer was created | 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional] 
**default_payment_method_id** | **str** | The identifier for the default payment method. | [optional] 

## Example

```python
from hyperswitch.models.customer_response import CustomerResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerResponse from a JSON string
customer_response_instance = CustomerResponse.from_json(json)
# print the JSON string representation of the object
print(CustomerResponse.to_json())

# convert the object into a dict
customer_response_dict = customer_response_instance.to_dict()
# create an instance of CustomerResponse from a dict
customer_response_from_dict = CustomerResponse.from_dict(customer_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


