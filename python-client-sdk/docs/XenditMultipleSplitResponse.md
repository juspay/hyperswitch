# XenditMultipleSplitResponse

Fee information charged on the payment being collected via xendit

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**split_rule_id** | **str** | Identifier for split rule created for the payment | 
**for_user_id** | **str** | The sub-account user-id that you want to make this transaction for. | [optional] 
**name** | **str** | Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types. | 
**description** | **str** | Description to identify fee rule | 
**routes** | [**List[XenditSplitRoute]**](XenditSplitRoute.md) | Array of objects that define how the platform wants to route the fees and to which accounts. | 

## Example

```python
from hyperswitch.models.xendit_multiple_split_response import XenditMultipleSplitResponse

# TODO update the JSON string below
json = "{}"
# create an instance of XenditMultipleSplitResponse from a JSON string
xendit_multiple_split_response_instance = XenditMultipleSplitResponse.from_json(json)
# print the JSON string representation of the object
print(XenditMultipleSplitResponse.to_json())

# convert the object into a dict
xendit_multiple_split_response_dict = xendit_multiple_split_response_instance.to_dict()
# create an instance of XenditMultipleSplitResponse from a dict
xendit_multiple_split_response_from_dict = XenditMultipleSplitResponse.from_dict(xendit_multiple_split_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


