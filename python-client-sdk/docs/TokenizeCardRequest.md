# TokenizeCardRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_card_number** | **str** | Card Number | 
**card_expiry_month** | **str** | Card Expiry Month | 
**card_expiry_year** | **str** | Card Expiry Year | 
**card_cvc** | **str** | The CVC number for the card | [optional] 
**card_holder_name** | **str** | Card Holder Name | [optional] 
**nick_name** | **str** | Card Holder&#39;s Nick Name | [optional] 
**card_issuing_country** | **str** | Card Issuing Country | [optional] 
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**card_issuer** | **str** | Issuer Bank for Card | [optional] 
**card_type** | [**CardType**](CardType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.tokenize_card_request import TokenizeCardRequest

# TODO update the JSON string below
json = "{}"
# create an instance of TokenizeCardRequest from a JSON string
tokenize_card_request_instance = TokenizeCardRequest.from_json(json)
# print the JSON string representation of the object
print(TokenizeCardRequest.to_json())

# convert the object into a dict
tokenize_card_request_dict = tokenize_card_request_instance.to_dict()
# create an instance of TokenizeCardRequest from a dict
tokenize_card_request_from_dict = TokenizeCardRequest.from_dict(tokenize_card_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


