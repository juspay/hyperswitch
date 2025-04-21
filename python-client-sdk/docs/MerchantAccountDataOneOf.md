# MerchantAccountDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**iban** | [**MerchantAccountDataOneOfIban**](MerchantAccountDataOneOfIban.md) |  | 

## Example

```python
from hyperswitch.models.merchant_account_data_one_of import MerchantAccountDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountDataOneOf from a JSON string
merchant_account_data_one_of_instance = MerchantAccountDataOneOf.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountDataOneOf.to_json())

# convert the object into a dict
merchant_account_data_one_of_dict = merchant_account_data_one_of_instance.to_dict()
# create an instance of MerchantAccountDataOneOf from a dict
merchant_account_data_one_of_from_dict = MerchantAccountDataOneOf.from_dict(merchant_account_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


