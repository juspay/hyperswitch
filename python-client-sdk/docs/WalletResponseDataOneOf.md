# WalletResponseDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**apple_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 

## Example

```python
from hyperswitch.models.wallet_response_data_one_of import WalletResponseDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of WalletResponseDataOneOf from a JSON string
wallet_response_data_one_of_instance = WalletResponseDataOneOf.from_json(json)
# print the JSON string representation of the object
print(WalletResponseDataOneOf.to_json())

# convert the object into a dict
wallet_response_data_one_of_dict = wallet_response_data_one_of_instance.to_dict()
# create an instance of WalletResponseDataOneOf from a dict
wallet_response_data_one_of_from_dict = WalletResponseDataOneOf.from_dict(wallet_response_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


