# CardAdditionalData

Masked payout method details for card payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_issuer** | **str** | Issuer of the card | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_type** | **str** | Card type, can be either &#x60;credit&#x60; or &#x60;debit&#x60; | [optional] 
**card_issuing_country** | **str** | Card issuing country | [optional] 
**bank_code** | **str** | Code for Card issuing bank | [optional] 
**last4** | **str** | Last 4 digits of the card number | [optional] 
**card_isin** | **str** | The ISIN of the card | [optional] 
**card_extended_bin** | **str** | Extended bin of card, contains the first 8 digits of card number | [optional] 
**card_exp_month** | **str** | Card expiry month | 
**card_exp_year** | **str** | Card expiry year | 
**card_holder_name** | **str** | Card holder name | 

## Example

```python
from hyperswitch.models.card_additional_data import CardAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of CardAdditionalData from a JSON string
card_additional_data_instance = CardAdditionalData.from_json(json)
# print the JSON string representation of the object
print(CardAdditionalData.to_json())

# convert the object into a dict
card_additional_data_dict = card_additional_data_instance.to_dict()
# create an instance of CardAdditionalData from a dict
card_additional_data_from_dict = CardAdditionalData.from_dict(card_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


