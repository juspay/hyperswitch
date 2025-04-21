# WalletResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**apple_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 
**google_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 
**samsung_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 

## Example

```python
from hyperswitch.models.wallet_response import WalletResponse

# TODO update the JSON string below
json = "{}"
# create an instance of WalletResponse from a JSON string
wallet_response_instance = WalletResponse.from_json(json)
# print the JSON string representation of the object
print(WalletResponse.to_json())

# convert the object into a dict
wallet_response_dict = wallet_response_instance.to_dict()
# create an instance of WalletResponse from a dict
wallet_response_from_dict = WalletResponse.from_dict(wallet_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


