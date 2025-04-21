# CardRedirectData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**knet** | **object** |  | 
**benefit** | **object** |  | 
**momo_atm** | **object** |  | 
**card_redirect** | **object** |  | 

## Example

```python
from hyperswitch.models.card_redirect_data import CardRedirectData

# TODO update the JSON string below
json = "{}"
# create an instance of CardRedirectData from a JSON string
card_redirect_data_instance = CardRedirectData.from_json(json)
# print the JSON string representation of the object
print(CardRedirectData.to_json())

# convert the object into a dict
card_redirect_data_dict = card_redirect_data_instance.to_dict()
# create an instance of CardRedirectData from a dict
card_redirect_data_from_dict = CardRedirectData.from_dict(card_redirect_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


