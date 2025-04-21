# PayPalWalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**token** | **str** | Token generated for the Apple pay | 

## Example

```python
from hyperswitch.models.pay_pal_wallet_data import PayPalWalletData

# TODO update the JSON string below
json = "{}"
# create an instance of PayPalWalletData from a JSON string
pay_pal_wallet_data_instance = PayPalWalletData.from_json(json)
# print the JSON string representation of the object
print(PayPalWalletData.to_json())

# convert the object into a dict
pay_pal_wallet_data_dict = pay_pal_wallet_data_instance.to_dict()
# create an instance of PayPalWalletData from a dict
pay_pal_wallet_data_from_dict = PayPalWalletData.from_dict(pay_pal_wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


