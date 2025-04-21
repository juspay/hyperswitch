# DokuBillingDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**first_name** | **str** | The billing first name for Doku | [optional] 
**last_name** | **str** | The billing second name for Doku | [optional] 
**email** | **str** | The Email ID for Doku billing | [optional] 

## Example

```python
from hyperswitch.models.doku_billing_details import DokuBillingDetails

# TODO update the JSON string below
json = "{}"
# create an instance of DokuBillingDetails from a JSON string
doku_billing_details_instance = DokuBillingDetails.from_json(json)
# print the JSON string representation of the object
print(DokuBillingDetails.to_json())

# convert the object into a dict
doku_billing_details_dict = doku_billing_details_instance.to_dict()
# create an instance of DokuBillingDetails from a dict
doku_billing_details_from_dict = DokuBillingDetails.from_dict(doku_billing_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


