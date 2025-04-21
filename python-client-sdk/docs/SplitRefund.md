# SplitRefund

Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe_split_refund** | [**StripeSplitRefundRequest**](StripeSplitRefundRequest.md) |  | 
**adyen_split_refund** | [**AdyenSplitData**](AdyenSplitData.md) |  | 
**xendit_split_refund** | [**XenditSplitSubMerchantData**](XenditSplitSubMerchantData.md) |  | 

## Example

```python
from hyperswitch.models.split_refund import SplitRefund

# TODO update the JSON string below
json = "{}"
# create an instance of SplitRefund from a JSON string
split_refund_instance = SplitRefund.from_json(json)
# print the JSON string representation of the object
print(SplitRefund.to_json())

# convert the object into a dict
split_refund_dict = split_refund_instance.to_dict()
# create an instance of SplitRefund from a dict
split_refund_from_dict = SplitRefund.from_dict(split_refund_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


