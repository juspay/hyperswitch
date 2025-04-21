# CardRedirectResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**knet** | **object** |  | 
**benefit** | **object** |  | 
**momo_atm** | **object** |  | 
**card_redirect** | **object** |  | 

## Example

```python
from hyperswitch.models.card_redirect_response import CardRedirectResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CardRedirectResponse from a JSON string
card_redirect_response_instance = CardRedirectResponse.from_json(json)
# print the JSON string representation of the object
print(CardRedirectResponse.to_json())

# convert the object into a dict
card_redirect_response_dict = card_redirect_response_instance.to_dict()
# create an instance of CardRedirectResponse from a dict
card_redirect_response_from_dict = CardRedirectResponse.from_dict(card_redirect_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


