# RefundListRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**start_time** | **datetime** | The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed | 
**end_time** | **datetime** | The end time to filter payments list or to get list of filters. If not passed the default time is now | [optional] 
**payment_id** | **str** | The identifier for the payment | [optional] 
**refund_id** | **str** | The identifier for the refund | [optional] 
**profile_id** | **str** | The identifier for business profile | [optional] 
**limit** | **int** | Limit on the number of objects to return | [optional] 
**offset** | **int** | The starting point within a list of objects | [optional] 
**amount_filter** | [**AmountFilter**](AmountFilter.md) |  | [optional] 
**connector** | **List[str]** | The list of connectors to filter refunds list | [optional] 
**merchant_connector_id** | **List[str]** | The list of merchant connector ids to filter the refunds list for selected label | [optional] 
**currency** | [**List[Currency]**](Currency.md) | The list of currencies to filter refunds list | [optional] 
**refund_status** | [**List[RefundStatus]**](RefundStatus.md) | The list of refund statuses to filter refunds list | [optional] 

## Example

```python
from hyperswitch.models.refund_list_request import RefundListRequest

# TODO update the JSON string below
json = "{}"
# create an instance of RefundListRequest from a JSON string
refund_list_request_instance = RefundListRequest.from_json(json)
# print the JSON string representation of the object
print(RefundListRequest.to_json())

# convert the object into a dict
refund_list_request_dict = refund_list_request_instance.to_dict()
# create an instance of RefundListRequest from a dict
refund_list_request_from_dict = RefundListRequest.from_dict(refund_list_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


