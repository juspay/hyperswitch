# WalletAdditionalData

Masked payout method details for wallet payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **str** | Email linked with paypal account | [optional] 
**telephone_number** | **str** | mobile number linked to venmo account | [optional] 
**paypal_id** | **str** | id of the paypal account | [optional] 

## Example

```python
from hyperswitch.models.wallet_additional_data import WalletAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of WalletAdditionalData from a JSON string
wallet_additional_data_instance = WalletAdditionalData.from_json(json)
# print the JSON string representation of the object
print(WalletAdditionalData.to_json())

# convert the object into a dict
wallet_additional_data_dict = wallet_additional_data_instance.to_dict()
# create an instance of WalletAdditionalData from a dict
wallet_additional_data_from_dict = WalletAdditionalData.from_dict(wallet_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


