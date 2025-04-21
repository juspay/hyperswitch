# XenditSplitRoute

Fee information to be charged on the payment being collected via xendit

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**flat_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | [optional] 
**percent_amount** | **int** | Amount of payments to be split, using a percent rate as unit | [optional] 
**currency** | [**Currency**](Currency.md) |  | 
**destination_account_id** | **str** | ID of the destination account where the amount will be routed to | 
**reference_id** | **str** | Reference ID which acts as an identifier of the route itself | 

## Example

```python
from hyperswitch.models.xendit_split_route import XenditSplitRoute

# TODO update the JSON string below
json = "{}"
# create an instance of XenditSplitRoute from a JSON string
xendit_split_route_instance = XenditSplitRoute.from_json(json)
# print the JSON string representation of the object
print(XenditSplitRoute.to_json())

# convert the object into a dict
xendit_split_route_dict = xendit_split_route_instance.to_dict()
# create an instance of XenditSplitRoute from a dict
xendit_split_route_from_dict = XenditSplitRoute.from_dict(xendit_split_route_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


