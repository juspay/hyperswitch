# GiftCardResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**givex** | [**GivexGiftCardAdditionalData**](GivexGiftCardAdditionalData.md) |  | 
**pay_safe_card** | **object** |  | 

## Example

```python
from hyperswitch.models.gift_card_response import GiftCardResponse

# TODO update the JSON string below
json = "{}"
# create an instance of GiftCardResponse from a JSON string
gift_card_response_instance = GiftCardResponse.from_json(json)
# print the JSON string representation of the object
print(GiftCardResponse.to_json())

# convert the object into a dict
gift_card_response_dict = gift_card_response_instance.to_dict()
# create an instance of GiftCardResponse from a dict
gift_card_response_from_dict = GiftCardResponse.from_dict(gift_card_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


