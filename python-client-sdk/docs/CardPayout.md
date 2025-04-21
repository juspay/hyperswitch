# CardPayout


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_number** | **str** | The card number | 
**expiry_month** | **str** | The card&#39;s expiry month | 
**expiry_year** | **str** | The card&#39;s expiry year | 
**card_holder_name** | **str** | The card holder&#39;s name | 

## Example

```python
from hyperswitch.models.card_payout import CardPayout

# TODO update the JSON string below
json = "{}"
# create an instance of CardPayout from a JSON string
card_payout_instance = CardPayout.from_json(json)
# print the JSON string representation of the object
print(CardPayout.to_json())

# convert the object into a dict
card_payout_dict = card_payout_instance.to_dict()
# create an instance of CardPayout from a dict
card_payout_from_dict = CardPayout.from_dict(card_payout_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


