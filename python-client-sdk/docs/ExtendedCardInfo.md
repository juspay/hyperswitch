# ExtendedCardInfo


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

## Example

```python
from hyperswitch.models.extended_card_info import ExtendedCardInfo

# TODO update the JSON string below
json = "{}"
# create an instance of ExtendedCardInfo from a JSON string
extended_card_info_instance = ExtendedCardInfo.from_json(json)
# print the JSON string representation of the object
print(ExtendedCardInfo.to_json())

# convert the object into a dict
extended_card_info_dict = extended_card_info_instance.to_dict()
# create an instance of ExtendedCardInfo from a dict
extended_card_info_from_dict = ExtendedCardInfo.from_dict(extended_card_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


