# WalletResponseDataOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**google_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 

## Example

```python
from hyperswitch.models.wallet_response_data_one_of1 import WalletResponseDataOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of WalletResponseDataOneOf1 from a JSON string
wallet_response_data_one_of1_instance = WalletResponseDataOneOf1.from_json(json)
# print the JSON string representation of the object
print(WalletResponseDataOneOf1.to_json())

# convert the object into a dict
wallet_response_data_one_of1_dict = wallet_response_data_one_of1_instance.to_dict()
# create an instance of WalletResponseDataOneOf1 from a dict
wallet_response_data_one_of1_from_dict = WalletResponseDataOneOf1.from_dict(wallet_response_data_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


