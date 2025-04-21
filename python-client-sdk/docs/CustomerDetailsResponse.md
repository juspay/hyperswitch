# CustomerDetailsResponse

Details of customer attached to this payment

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** | The identifier for the customer. | [optional] 
**name** | **str** | The customer&#39;s name | [optional] 
**email** | **str** | The customer&#39;s email address | [optional] 
**phone** | **str** | The customer&#39;s phone number | [optional] 
**phone_country_code** | **str** | The country code for the customer&#39;s phone number | [optional] 

## Example

```python
from hyperswitch.models.customer_details_response import CustomerDetailsResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerDetailsResponse from a JSON string
customer_details_response_instance = CustomerDetailsResponse.from_json(json)
# print the JSON string representation of the object
print(CustomerDetailsResponse.to_json())

# convert the object into a dict
customer_details_response_dict = customer_details_response_instance.to_dict()
# create an instance of CustomerDetailsResponse from a dict
customer_details_response_from_dict = CustomerDetailsResponse.from_dict(customer_details_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


