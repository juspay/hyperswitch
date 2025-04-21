# MerchantRecipientData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_recipient_id** | **str** |  | 
**wallet_id** | **str** |  | 
**account_data** | [**MerchantAccountData**](MerchantAccountData.md) |  | 

## Example

```python
from hyperswitch.models.merchant_recipient_data import MerchantRecipientData

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantRecipientData from a JSON string
merchant_recipient_data_instance = MerchantRecipientData.from_json(json)
# print the JSON string representation of the object
print(MerchantRecipientData.to_json())

# convert the object into a dict
merchant_recipient_data_dict = merchant_recipient_data_instance.to_dict()
# create an instance of MerchantRecipientData from a dict
merchant_recipient_data_from_dict = MerchantRecipientData.from_dict(merchant_recipient_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


