# WalletResponseData

Hyperswitch supports SDK integration with Apple Pay and Google Pay wallets. For other wallets, we integrate with their respective connectors, redirecting the customer to the connector for wallet payments. As a result, we don’t receive any payment method data in the confirm call for payments made through other wallets.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**apple_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 
**google_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 
**samsung_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 

## Example

```python
from hyperswitch.models.wallet_response_data import WalletResponseData

# TODO update the JSON string below
json = "{}"
# create an instance of WalletResponseData from a JSON string
wallet_response_data_instance = WalletResponseData.from_json(json)
# print the JSON string representation of the object
print(WalletResponseData.to_json())

# convert the object into a dict
wallet_response_data_dict = wallet_response_data_instance.to_dict()
# create an instance of WalletResponseData from a dict
wallet_response_data_from_dict = WalletResponseData.from_dict(wallet_response_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


