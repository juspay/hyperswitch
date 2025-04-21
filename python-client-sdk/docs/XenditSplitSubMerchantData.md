# XenditSplitSubMerchantData

Fee information to be charged on the payment being collected for sub-merchant via xendit

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**for_user_id** | **str** | The sub-account user-id that you want to make this transaction for. | 

## Example

```python
from hyperswitch.models.xendit_split_sub_merchant_data import XenditSplitSubMerchantData

# TODO update the JSON string below
json = "{}"
# create an instance of XenditSplitSubMerchantData from a JSON string
xendit_split_sub_merchant_data_instance = XenditSplitSubMerchantData.from_json(json)
# print the JSON string representation of the object
print(XenditSplitSubMerchantData.to_json())

# convert the object into a dict
xendit_split_sub_merchant_data_dict = xendit_split_sub_merchant_data_instance.to_dict()
# create an instance of XenditSplitSubMerchantData from a dict
xendit_split_sub_merchant_data_from_dict = XenditSplitSubMerchantData.from_dict(xendit_split_sub_merchant_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


