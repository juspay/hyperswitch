# CardTokenResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_holder_name** | **str** | The card holder&#39;s name | 

## Example

```python
from hyperswitch.models.card_token_response import CardTokenResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CardTokenResponse from a JSON string
card_token_response_instance = CardTokenResponse.from_json(json)
# print the JSON string representation of the object
print(CardTokenResponse.to_json())

# convert the object into a dict
card_token_response_dict = card_token_response_instance.to_dict()
# create an instance of CardTokenResponse from a dict
card_token_response_from_dict = CardTokenResponse.from_dict(card_token_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


