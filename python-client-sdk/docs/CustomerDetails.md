# CustomerDetails

Passing this object creates a new customer or attaches an existing customer to the payment

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** | The identifier for the customer. | 
**name** | **str** | The customer&#39;s name | [optional] 
**email** | **str** | The customer&#39;s email address | [optional] 
**phone** | **str** | The customer&#39;s phone number | [optional] 
**phone_country_code** | **str** | The country code for the customer&#39;s phone number | [optional] 

## Example

```python
from hyperswitch.models.customer_details import CustomerDetails

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerDetails from a JSON string
customer_details_instance = CustomerDetails.from_json(json)
# print the JSON string representation of the object
print(CustomerDetails.to_json())

# convert the object into a dict
customer_details_dict = customer_details_instance.to_dict()
# create an instance of CustomerDetails from a dict
customer_details_from_dict = CustomerDetails.from_dict(customer_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


