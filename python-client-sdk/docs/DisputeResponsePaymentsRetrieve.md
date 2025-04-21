# DisputeResponsePaymentsRetrieve


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**dispute_id** | **str** | The identifier for dispute | 
**dispute_stage** | [**DisputeStage**](DisputeStage.md) |  | 
**dispute_status** | [**DisputeStatus**](DisputeStatus.md) |  | 
**connector_status** | **str** | Status of the dispute sent by connector | 
**connector_dispute_id** | **str** | Dispute id sent by connector | 
**connector_reason** | **str** | Reason of dispute sent by connector | [optional] 
**connector_reason_code** | **str** | Reason code of dispute sent by connector | [optional] 
**challenge_required_by** | **datetime** | Evidence deadline of dispute sent by connector | [optional] 
**connector_created_at** | **datetime** | Dispute created time sent by connector | [optional] 
**connector_updated_at** | **datetime** | Dispute updated time sent by connector | [optional] 
**created_at** | **datetime** | Time at which dispute is received | 

## Example

```python
from hyperswitch.models.dispute_response_payments_retrieve import DisputeResponsePaymentsRetrieve

# TODO update the JSON string below
json = "{}"
# create an instance of DisputeResponsePaymentsRetrieve from a JSON string
dispute_response_payments_retrieve_instance = DisputeResponsePaymentsRetrieve.from_json(json)
# print the JSON string representation of the object
print(DisputeResponsePaymentsRetrieve.to_json())

# convert the object into a dict
dispute_response_payments_retrieve_dict = dispute_response_payments_retrieve_instance.to_dict()
# create an instance of DisputeResponsePaymentsRetrieve from a dict
dispute_response_payments_retrieve_from_dict = DisputeResponsePaymentsRetrieve.from_dict(dispute_response_payments_retrieve_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


