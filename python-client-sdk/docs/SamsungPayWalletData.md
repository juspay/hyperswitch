# SamsungPayWalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_credential** | [**SamsungPayWalletCredentials**](SamsungPayWalletCredentials.md) |  | 

## Example

```python
from hyperswitch.models.samsung_pay_wallet_data import SamsungPayWalletData

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPayWalletData from a JSON string
samsung_pay_wallet_data_instance = SamsungPayWalletData.from_json(json)
# print the JSON string representation of the object
print(SamsungPayWalletData.to_json())

# convert the object into a dict
samsung_pay_wallet_data_dict = samsung_pay_wallet_data_instance.to_dict()
# create an instance of SamsungPayWalletData from a dict
samsung_pay_wallet_data_from_dict = SamsungPayWalletData.from_dict(samsung_pay_wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


