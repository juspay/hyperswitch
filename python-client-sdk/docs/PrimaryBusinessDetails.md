# PrimaryBusinessDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | 
**business** | **str** |  | 

## Example

```python
from hyperswitch.models.primary_business_details import PrimaryBusinessDetails

# TODO update the JSON string below
json = "{}"
# create an instance of PrimaryBusinessDetails from a JSON string
primary_business_details_instance = PrimaryBusinessDetails.from_json(json)
# print the JSON string representation of the object
print(PrimaryBusinessDetails.to_json())

# convert the object into a dict
primary_business_details_dict = primary_business_details_instance.to_dict()
# create an instance of PrimaryBusinessDetails from a dict
primary_business_details_from_dict = PrimaryBusinessDetails.from_dict(primary_business_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


