# AcceptedCurrencies


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**list** | [**List[Currency]**](Currency.md) |  | 

## Example

```python
from hyperswitch.models.accepted_currencies import AcceptedCurrencies

# TODO update the JSON string below
json = "{}"
# create an instance of AcceptedCurrencies from a JSON string
accepted_currencies_instance = AcceptedCurrencies.from_json(json)
# print the JSON string representation of the object
print(AcceptedCurrencies.to_json())

# convert the object into a dict
accepted_currencies_dict = accepted_currencies_instance.to_dict()
# create an instance of AcceptedCurrencies from a dict
accepted_currencies_from_dict = AcceptedCurrencies.from_dict(accepted_currencies_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


