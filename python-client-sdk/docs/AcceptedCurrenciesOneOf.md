# AcceptedCurrenciesOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**list** | [**List[Currency]**](Currency.md) |  | 

## Example

```python
from hyperswitch.models.accepted_currencies_one_of import AcceptedCurrenciesOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of AcceptedCurrenciesOneOf from a JSON string
accepted_currencies_one_of_instance = AcceptedCurrenciesOneOf.from_json(json)
# print the JSON string representation of the object
print(AcceptedCurrenciesOneOf.to_json())

# convert the object into a dict
accepted_currencies_one_of_dict = accepted_currencies_one_of_instance.to_dict()
# create an instance of AcceptedCurrenciesOneOf from a dict
accepted_currencies_one_of_from_dict = AcceptedCurrenciesOneOf.from_dict(accepted_currencies_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


