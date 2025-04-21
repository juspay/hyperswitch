# Card


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_number** | **str** | The card number | 
**card_exp_month** | **str** | The card&#39;s expiry month | 
**card_exp_year** | **str** | The card&#39;s expiry year | 
**card_holder_name** | **str** | The card holder&#39;s name | 
**card_cvc** | **str** | The CVC number for the card | 
**card_issuer** | **str** | The name of the issuer of card | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_type** | **str** |  | [optional] 
**card_issuing_country** | **str** |  | [optional] 
**bank_code** | **str** |  | [optional] 
**nick_name** | **str** | The card holder&#39;s nick name | [optional] 

## Example

```python
from hyperswitch.models.card import Card

# TODO update the JSON string below
json = "{}"
# create an instance of Card from a JSON string
card_instance = Card.from_json(json)
# print the JSON string representation of the object
print(Card.to_json())

# convert the object into a dict
card_dict = card_instance.to_dict()
# create an instance of Card from a dict
card_from_dict = Card.from_dict(card_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


