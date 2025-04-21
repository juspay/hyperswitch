# GiftCardData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**givex** | [**GiftCardDetails**](GiftCardDetails.md) |  | 
**pay_safe_card** | **object** |  | 

## Example

```python
from hyperswitch.models.gift_card_data import GiftCardData

# TODO update the JSON string below
json = "{}"
# create an instance of GiftCardData from a JSON string
gift_card_data_instance = GiftCardData.from_json(json)
# print the JSON string representation of the object
print(GiftCardData.to_json())

# convert the object into a dict
gift_card_data_dict = gift_card_data_instance.to_dict()
# create an instance of GiftCardData from a dict
gift_card_data_from_dict = GiftCardData.from_dict(gift_card_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


