# CardToken


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_holder_name** | **str** | The card holder&#39;s name | 
**card_cvc** | **str** | The CVC number for the card | [optional] 

## Example

```python
from hyperswitch.models.card_token import CardToken

# TODO update the JSON string below
json = "{}"
# create an instance of CardToken from a JSON string
card_token_instance = CardToken.from_json(json)
# print the JSON string representation of the object
print(CardToken.to_json())

# convert the object into a dict
card_token_dict = card_token_instance.to_dict()
# create an instance of CardToken from a dict
card_token_from_dict = CardToken.from_dict(card_token_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


