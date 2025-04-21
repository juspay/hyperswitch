# GiftCardDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**number** | **str** | The gift card number | 
**cvc** | **str** | The card verification code. | 

## Example

```python
from hyperswitch.models.gift_card_details import GiftCardDetails

# TODO update the JSON string below
json = "{}"
# create an instance of GiftCardDetails from a JSON string
gift_card_details_instance = GiftCardDetails.from_json(json)
# print the JSON string representation of the object
print(GiftCardDetails.to_json())

# convert the object into a dict
gift_card_details_dict = gift_card_details_instance.to_dict()
# create an instance of GiftCardDetails from a dict
gift_card_details_from_dict = GiftCardDetails.from_dict(gift_card_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


