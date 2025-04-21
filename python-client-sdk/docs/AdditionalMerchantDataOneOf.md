# AdditionalMerchantDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking_recipient_data** | [**MerchantRecipientData**](MerchantRecipientData.md) |  | 

## Example

```python
from hyperswitch.models.additional_merchant_data_one_of import AdditionalMerchantDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of AdditionalMerchantDataOneOf from a JSON string
additional_merchant_data_one_of_instance = AdditionalMerchantDataOneOf.from_json(json)
# print the JSON string representation of the object
print(AdditionalMerchantDataOneOf.to_json())

# convert the object into a dict
additional_merchant_data_one_of_dict = additional_merchant_data_one_of_instance.to_dict()
# create an instance of AdditionalMerchantDataOneOf from a dict
additional_merchant_data_one_of_from_dict = AdditionalMerchantDataOneOf.from_dict(additional_merchant_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


