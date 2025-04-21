# GiftCardAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**givex** | [**GivexGiftCardAdditionalData**](GivexGiftCardAdditionalData.md) |  | 
**pay_safe_card** | **object** |  | 

## Example

```python
from hyperswitch.models.gift_card_additional_data import GiftCardAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of GiftCardAdditionalData from a JSON string
gift_card_additional_data_instance = GiftCardAdditionalData.from_json(json)
# print the JSON string representation of the object
print(GiftCardAdditionalData.to_json())

# convert the object into a dict
gift_card_additional_data_dict = gift_card_additional_data_instance.to_dict()
# create an instance of GiftCardAdditionalData from a dict
gift_card_additional_data_from_dict = GiftCardAdditionalData.from_dict(gift_card_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


