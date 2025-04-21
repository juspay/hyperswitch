# AchBillingDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **str** | The Email ID for ACH billing | [optional] 

## Example

```python
from hyperswitch.models.ach_billing_details import AchBillingDetails

# TODO update the JSON string below
json = "{}"
# create an instance of AchBillingDetails from a JSON string
ach_billing_details_instance = AchBillingDetails.from_json(json)
# print the JSON string representation of the object
print(AchBillingDetails.to_json())

# convert the object into a dict
ach_billing_details_dict = ach_billing_details_instance.to_dict()
# create an instance of AchBillingDetails from a dict
ach_billing_details_from_dict = AchBillingDetails.from_dict(ach_billing_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


