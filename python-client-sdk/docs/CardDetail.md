# CardDetail


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_number** | **str** | Card Number | 
**card_exp_month** | **str** | Card Expiry Month | 
**card_exp_year** | **str** | Card Expiry Year | 
**card_holder_name** | **str** | Card Holder Name | 
**nick_name** | **str** | Card Holder&#39;s Nick Name | [optional] 
**card_issuing_country** | **str** | Card Issuing Country | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_issuer** | **str** | Issuer Bank for Card | [optional] 
**card_type** | **str** | Card Type | [optional] 

## Example

```python
from hyperswitch.models.card_detail import CardDetail

# TODO update the JSON string below
json = "{}"
# create an instance of CardDetail from a JSON string
card_detail_instance = CardDetail.from_json(json)
# print the JSON string representation of the object
print(CardDetail.to_json())

# convert the object into a dict
card_detail_dict = card_detail_instance.to_dict()
# create an instance of CardDetail from a dict
card_detail_from_dict = CardDetail.from_dict(card_detail_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


