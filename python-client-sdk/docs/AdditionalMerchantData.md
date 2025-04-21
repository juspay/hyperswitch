# AdditionalMerchantData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking_recipient_data** | [**MerchantRecipientData**](MerchantRecipientData.md) |  | 

## Example

```python
from hyperswitch.models.additional_merchant_data import AdditionalMerchantData

# TODO update the JSON string below
json = "{}"
# create an instance of AdditionalMerchantData from a JSON string
additional_merchant_data_instance = AdditionalMerchantData.from_json(json)
# print the JSON string representation of the object
print(AdditionalMerchantData.to_json())

# convert the object into a dict
additional_merchant_data_dict = additional_merchant_data_instance.to_dict()
# create an instance of AdditionalMerchantData from a dict
additional_merchant_data_from_dict = AdditionalMerchantData.from_dict(additional_merchant_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


