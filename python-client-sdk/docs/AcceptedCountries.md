# AcceptedCountries

Object to filter the customer countries for which the payment method is displayed

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**list** | [**List[CountryAlpha2]**](CountryAlpha2.md) |  | 

## Example

```python
from hyperswitch.models.accepted_countries import AcceptedCountries

# TODO update the JSON string below
json = "{}"
# create an instance of AcceptedCountries from a JSON string
accepted_countries_instance = AcceptedCountries.from_json(json)
# print the JSON string representation of the object
print(AcceptedCountries.to_json())

# convert the object into a dict
accepted_countries_dict = accepted_countries_instance.to_dict()
# create an instance of AcceptedCountries from a dict
accepted_countries_from_dict = AcceptedCountries.from_dict(accepted_countries_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


