# GivexGiftCardAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**last4** | **str** | Last 4 digits of the gift card number | 

## Example

```python
from hyperswitch.models.givex_gift_card_additional_data import GivexGiftCardAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of GivexGiftCardAdditionalData from a JSON string
givex_gift_card_additional_data_instance = GivexGiftCardAdditionalData.from_json(json)
# print the JSON string representation of the object
print(GivexGiftCardAdditionalData.to_json())

# convert the object into a dict
givex_gift_card_additional_data_dict = givex_gift_card_additional_data_instance.to_dict()
# create an instance of GivexGiftCardAdditionalData from a dict
givex_gift_card_additional_data_from_dict = GivexGiftCardAdditionalData.from_dict(givex_gift_card_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


