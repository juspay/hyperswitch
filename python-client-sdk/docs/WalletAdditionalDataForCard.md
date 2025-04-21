# WalletAdditionalDataForCard


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**last4** | **str** | Last 4 digits of the card number | 
**card_network** | **str** | The information of the payment method | 
**type** | **str** | The type of payment method | [optional] 

## Example

```python
from hyperswitch.models.wallet_additional_data_for_card import WalletAdditionalDataForCard

# TODO update the JSON string below
json = "{}"
# create an instance of WalletAdditionalDataForCard from a JSON string
wallet_additional_data_for_card_instance = WalletAdditionalDataForCard.from_json(json)
# print the JSON string representation of the object
print(WalletAdditionalDataForCard.to_json())

# convert the object into a dict
wallet_additional_data_for_card_dict = wallet_additional_data_for_card_instance.to_dict()
# create an instance of WalletAdditionalDataForCard from a dict
wallet_additional_data_for_card_from_dict = WalletAdditionalDataForCard.from_dict(wallet_additional_data_for_card_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


