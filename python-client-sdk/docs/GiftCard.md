# GiftCard


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**gift_card** | [**GiftCardData**](GiftCardData.md) |  | 

## Example

```python
from hyperswitch.models.gift_card import GiftCard

# TODO update the JSON string below
json = "{}"
# create an instance of GiftCard from a JSON string
gift_card_instance = GiftCard.from_json(json)
# print the JSON string representation of the object
print(GiftCard.to_json())

# convert the object into a dict
gift_card_dict = gift_card_instance.to_dict()
# create an instance of GiftCard from a dict
gift_card_from_dict = GiftCard.from_dict(gift_card_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


