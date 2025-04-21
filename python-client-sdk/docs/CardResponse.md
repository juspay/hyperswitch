# CardResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**last4** | **str** |  | [optional] 
**card_type** | **str** |  | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_issuer** | **str** |  | [optional] 
**card_issuing_country** | **str** |  | [optional] 
**card_isin** | **str** |  | [optional] 
**card_extended_bin** | **str** |  | [optional] 
**card_exp_month** | **str** |  | [optional] 
**card_exp_year** | **str** |  | [optional] 
**card_holder_name** | **str** |  | [optional] 
**payment_checks** | **object** |  | [optional] 
**authentication_data** | **object** |  | [optional] 

## Example

```python
from hyperswitch.models.card_response import CardResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CardResponse from a JSON string
card_response_instance = CardResponse.from_json(json)
# print the JSON string representation of the object
print(CardResponse.to_json())

# convert the object into a dict
card_response_dict = card_response_instance.to_dict()
# create an instance of CardResponse from a dict
card_response_from_dict = CardResponse.from_dict(card_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


