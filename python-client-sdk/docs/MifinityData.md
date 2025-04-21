# MifinityData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**date_of_birth** | **date** |  | 
**language_preference** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.mifinity_data import MifinityData

# TODO update the JSON string below
json = "{}"
# create an instance of MifinityData from a JSON string
mifinity_data_instance = MifinityData.from_json(json)
# print the JSON string representation of the object
print(MifinityData.to_json())

# convert the object into a dict
mifinity_data_dict = mifinity_data_instance.to_dict()
# create an instance of MifinityData from a dict
mifinity_data_from_dict = MifinityData.from_dict(mifinity_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


