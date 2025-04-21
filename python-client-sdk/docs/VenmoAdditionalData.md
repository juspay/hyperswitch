# VenmoAdditionalData

Masked payout method details for venmo wallet payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**telephone_number** | **str** | mobile number linked to venmo account | [optional] 

## Example

```python
from hyperswitch.models.venmo_additional_data import VenmoAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of VenmoAdditionalData from a JSON string
venmo_additional_data_instance = VenmoAdditionalData.from_json(json)
# print the JSON string representation of the object
print(VenmoAdditionalData.to_json())

# convert the object into a dict
venmo_additional_data_dict = venmo_additional_data_instance.to_dict()
# create an instance of VenmoAdditionalData from a dict
venmo_additional_data_from_dict = VenmoAdditionalData.from_dict(venmo_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


