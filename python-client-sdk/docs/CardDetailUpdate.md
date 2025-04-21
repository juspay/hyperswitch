# CardDetailUpdate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_exp_month** | **str** | Card Expiry Month | 
**card_exp_year** | **str** | Card Expiry Year | 
**card_holder_name** | **str** | Card Holder Name | 
**nick_name** | **str** | Card Holder&#39;s Nick Name | [optional] 

## Example

```python
from hyperswitch.models.card_detail_update import CardDetailUpdate

# TODO update the JSON string below
json = "{}"
# create an instance of CardDetailUpdate from a JSON string
card_detail_update_instance = CardDetailUpdate.from_json(json)
# print the JSON string representation of the object
print(CardDetailUpdate.to_json())

# convert the object into a dict
card_detail_update_dict = card_detail_update_instance.to_dict()
# create an instance of CardDetailUpdate from a dict
card_detail_update_from_dict = CardDetailUpdate.from_dict(card_detail_update_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


