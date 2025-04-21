# WalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ali_pay_qr** | **object** |  | 
**ali_pay_redirect** | **object** |  | 
**ali_pay_hk_redirect** | **object** |  | 
**amazon_pay_redirect** | **object** |  | 
**momo_redirect** | **object** |  | 
**kakao_pay_redirect** | **object** |  | 
**go_pay_redirect** | **object** |  | 
**gcash_redirect** | **object** |  | 
**apple_pay** | [**ApplePayWalletData**](ApplePayWalletData.md) |  | 
**apple_pay_redirect** | **object** |  | 
**apple_pay_third_party_sdk** | **object** |  | 
**dana_redirect** | **object** | Wallet data for DANA redirect flow | 
**google_pay** | [**GooglePayWalletData**](GooglePayWalletData.md) |  | 
**google_pay_redirect** | **object** |  | 
**google_pay_third_party_sdk** | **object** |  | 
**mb_way_redirect** | [**MbWayRedirection**](MbWayRedirection.md) |  | 
**mobile_pay_redirect** | **object** |  | 
**paypal_redirect** | [**PaypalRedirection**](PaypalRedirection.md) |  | 
**paypal_sdk** | [**PayPalWalletData**](PayPalWalletData.md) |  | 
**paze** | [**PazeWalletData**](PazeWalletData.md) |  | 
**samsung_pay** | [**SamsungPayWalletData**](SamsungPayWalletData.md) |  | 
**twint_redirect** | **object** | Wallet data for Twint Redirection | 
**vipps_redirect** | **object** | Wallet data for Vipps Redirection | 
**touch_n_go_redirect** | **object** |  | 
**we_chat_pay_redirect** | **object** |  | 
**we_chat_pay_qr** | **object** |  | 
**cashapp_qr** | **object** |  | 
**swish_qr** | **object** |  | 
**mifinity** | [**MifinityData**](MifinityData.md) |  | 

## Example

```python
from hyperswitch.models.wallet_data import WalletData

# TODO update the JSON string below
json = "{}"
# create an instance of WalletData from a JSON string
wallet_data_instance = WalletData.from_json(json)
# print the JSON string representation of the object
print(WalletData.to_json())

# convert the object into a dict
wallet_data_dict = wallet_data_instance.to_dict()
# create an instance of WalletData from a dict
wallet_data_from_dict = WalletData.from_dict(wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


