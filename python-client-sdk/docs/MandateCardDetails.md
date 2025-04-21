# MandateCardDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**last4_digits** | **str** | The last 4 digits of card | [optional] 
**card_exp_month** | **str** | The expiry month of card | [optional] 
**card_exp_year** | **str** | The expiry year of card | [optional] 
**card_holder_name** | **str** | The card holder name | [optional] 
**card_token** | **str** | The token from card locker | [optional] 
**scheme** | **str** | The card scheme network for the particular card | [optional] 
**issuer_country** | **str** | The country code in in which the card was issued | [optional] 
**card_fingerprint** | **str** | A unique identifier alias to identify a particular card | [optional] 
**card_isin** | **str** | The first 6 digits of card | [optional] 
**card_issuer** | **str** | The bank that issued the card | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_type** | **str** | The type of the payment card | [optional] 
**nick_name** | **str** | The nick_name of the card holder | [optional] 

## Example

```python
from hyperswitch.models.mandate_card_details import MandateCardDetails

# TODO update the JSON string below
json = "{}"
# create an instance of MandateCardDetails from a JSON string
mandate_card_details_instance = MandateCardDetails.from_json(json)
# print the JSON string representation of the object
print(MandateCardDetails.to_json())

# convert the object into a dict
mandate_card_details_dict = mandate_card_details_instance.to_dict()
# create an instance of MandateCardDetails from a dict
mandate_card_details_from_dict = MandateCardDetails.from_dict(mandate_card_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


