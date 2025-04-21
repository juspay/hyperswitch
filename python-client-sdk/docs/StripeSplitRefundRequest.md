# StripeSplitRefundRequest

Charge specific fields for controlling the revert of funds from either platform or connected account for Stripe. Check sub-fields for more details.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**revert_platform_fee** | **bool** | Toggle for reverting the application fee that was collected for the payment. If set to false, the funds are pulled from the destination account. | [optional] 
**revert_transfer** | **bool** | Toggle for reverting the transfer that was made during the charge. If set to false, the funds are pulled from the main platform&#39;s account. | [optional] 

## Example

```python
from hyperswitch.models.stripe_split_refund_request import StripeSplitRefundRequest

# TODO update the JSON string below
json = "{}"
# create an instance of StripeSplitRefundRequest from a JSON string
stripe_split_refund_request_instance = StripeSplitRefundRequest.from_json(json)
# print the JSON string representation of the object
print(StripeSplitRefundRequest.to_json())

# convert the object into a dict
stripe_split_refund_request_dict = stripe_split_refund_request_instance.to_dict()
# create an instance of StripeSplitRefundRequest from a dict
stripe_split_refund_request_from_dict = StripeSplitRefundRequest.from_dict(stripe_split_refund_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


