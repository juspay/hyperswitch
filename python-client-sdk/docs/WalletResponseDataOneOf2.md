# WalletResponseDataOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**samsung_pay** | [**WalletAdditionalDataForCard**](WalletAdditionalDataForCard.md) |  | 

## Example

```python
from hyperswitch.models.wallet_response_data_one_of2 import WalletResponseDataOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of WalletResponseDataOneOf2 from a JSON string
wallet_response_data_one_of2_instance = WalletResponseDataOneOf2.from_json(json)
# print the JSON string representation of the object
print(WalletResponseDataOneOf2.to_json())

# convert the object into a dict
wallet_response_data_one_of2_dict = wallet_response_data_one_of2_instance.to_dict()
# create an instance of WalletResponseDataOneOf2 from a dict
wallet_response_data_one_of2_from_dict = WalletResponseDataOneOf2.from_dict(wallet_response_data_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


