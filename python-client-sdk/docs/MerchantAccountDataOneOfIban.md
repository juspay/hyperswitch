# MerchantAccountDataOneOfIban


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**iban** | **str** |  | 
**name** | **str** |  | 
**connector_recipient_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_account_data_one_of_iban import MerchantAccountDataOneOfIban

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountDataOneOfIban from a JSON string
merchant_account_data_one_of_iban_instance = MerchantAccountDataOneOfIban.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountDataOneOfIban.to_json())

# convert the object into a dict
merchant_account_data_one_of_iban_dict = merchant_account_data_one_of_iban_instance.to_dict()
# create an instance of MerchantAccountDataOneOfIban from a dict
merchant_account_data_one_of_iban_from_dict = MerchantAccountDataOneOfIban.from_dict(merchant_account_data_one_of_iban_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


