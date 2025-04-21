# PhoneDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**number** | **str** | The contact number | [optional] 
**country_code** | **str** | The country code attached to the number | [optional] 

## Example

```python
from hyperswitch.models.phone_details import PhoneDetails

# TODO update the JSON string below
json = "{}"
# create an instance of PhoneDetails from a JSON string
phone_details_instance = PhoneDetails.from_json(json)
# print the JSON string representation of the object
print(PhoneDetails.to_json())

# convert the object into a dict
phone_details_dict = phone_details_instance.to_dict()
# create an instance of PhoneDetails from a dict
phone_details_from_dict = PhoneDetails.from_dict(phone_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


