# MerchantAccountDataOneOf1Bacs


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **str** |  | 
**sort_code** | **str** |  | 
**name** | **str** |  | 
**connector_recipient_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_account_data_one_of1_bacs import MerchantAccountDataOneOf1Bacs

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantAccountDataOneOf1Bacs from a JSON string
merchant_account_data_one_of1_bacs_instance = MerchantAccountDataOneOf1Bacs.from_json(json)
# print the JSON string representation of the object
print(MerchantAccountDataOneOf1Bacs.to_json())

# convert the object into a dict
merchant_account_data_one_of1_bacs_dict = merchant_account_data_one_of1_bacs_instance.to_dict()
# create an instance of MerchantAccountDataOneOf1Bacs from a dict
merchant_account_data_one_of1_bacs_from_dict = MerchantAccountDataOneOf1Bacs.from_dict(merchant_account_data_one_of1_bacs_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


