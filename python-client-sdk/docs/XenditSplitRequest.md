# XenditSplitRequest

Xendit Charge Request

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**multiple_splits** | [**XenditMultipleSplitRequest**](XenditMultipleSplitRequest.md) |  | 
**single_split** | [**XenditSplitSubMerchantData**](XenditSplitSubMerchantData.md) |  | 

## Example

```python
from hyperswitch.models.xendit_split_request import XenditSplitRequest

# TODO update the JSON string below
json = "{}"
# create an instance of XenditSplitRequest from a JSON string
xendit_split_request_instance = XenditSplitRequest.from_json(json)
# print the JSON string representation of the object
print(XenditSplitRequest.to_json())

# convert the object into a dict
xendit_split_request_dict = xendit_split_request_instance.to_dict()
# create an instance of XenditSplitRequest from a dict
xendit_split_request_from_dict = XenditSplitRequest.from_dict(xendit_split_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


