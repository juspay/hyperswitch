# SplitRefundOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe_split_refund** | [**StripeSplitRefundRequest**](StripeSplitRefundRequest.md) |  | 

## Example

```python
from hyperswitch.models.split_refund_one_of import SplitRefundOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of SplitRefundOneOf from a JSON string
split_refund_one_of_instance = SplitRefundOneOf.from_json(json)
# print the JSON string representation of the object
print(SplitRefundOneOf.to_json())

# convert the object into a dict
split_refund_one_of_dict = split_refund_one_of_instance.to_dict()
# create an instance of SplitRefundOneOf from a dict
split_refund_one_of_from_dict = SplitRefundOneOf.from_dict(split_refund_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


