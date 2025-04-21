# CardTokenAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_holder_name** | **str** | The card holder&#39;s name | 

## Example

```python
from hyperswitch.models.card_token_additional_data import CardTokenAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of CardTokenAdditionalData from a JSON string
card_token_additional_data_instance = CardTokenAdditionalData.from_json(json)
# print the JSON string representation of the object
print(CardTokenAdditionalData.to_json())

# convert the object into a dict
card_token_additional_data_dict = card_token_additional_data_instance.to_dict()
# create an instance of CardTokenAdditionalData from a dict
card_token_additional_data_from_dict = CardTokenAdditionalData.from_dict(card_token_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


