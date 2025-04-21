# CardDetailFromLocker


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**scheme** | **str** |  | [optional] 
**issuer_country** | **str** |  | [optional] 
**last4_digits** | **str** |  | [optional] 
**expiry_month** | **str** |  | [optional] 
**expiry_year** | **str** |  | [optional] 
**card_token** | **str** |  | [optional] 
**card_holder_name** | **str** |  | [optional] 
**card_fingerprint** | **str** |  | [optional] 
**nick_name** | **str** |  | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_isin** | **str** |  | [optional] 
**card_issuer** | **str** |  | [optional] 
**card_type** | **str** |  | [optional] 
**saved_to_locker** | **bool** |  | 

## Example

```python
from hyperswitch.models.card_detail_from_locker import CardDetailFromLocker

# TODO update the JSON string below
json = "{}"
# create an instance of CardDetailFromLocker from a JSON string
card_detail_from_locker_instance = CardDetailFromLocker.from_json(json)
# print the JSON string representation of the object
print(CardDetailFromLocker.to_json())

# convert the object into a dict
card_detail_from_locker_dict = card_detail_from_locker_instance.to_dict()
# create an instance of CardDetailFromLocker from a dict
card_detail_from_locker_from_dict = CardDetailFromLocker.from_dict(card_detail_from_locker_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


