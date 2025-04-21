# AcceptedCountriesOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**list** | [**List[CountryAlpha2]**](CountryAlpha2.md) |  | 

## Example

```python
from hyperswitch.models.accepted_countries_one_of import AcceptedCountriesOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of AcceptedCountriesOneOf from a JSON string
accepted_countries_one_of_instance = AcceptedCountriesOneOf.from_json(json)
# print the JSON string representation of the object
print(AcceptedCountriesOneOf.to_json())

# convert the object into a dict
accepted_countries_one_of_dict = accepted_countries_one_of_instance.to_dict()
# create an instance of AcceptedCountriesOneOf from a dict
accepted_countries_one_of_from_dict = AcceptedCountriesOneOf.from_dict(accepted_countries_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


