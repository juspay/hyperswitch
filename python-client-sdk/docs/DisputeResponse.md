# DisputeResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**dispute_id** | **str** | The identifier for dispute | 
**payment_id** | **str** | The identifier for payment_intent | 
**attempt_id** | **str** | The identifier for payment_attempt | 
**amount** | **str** | The dispute amount | 
**currency** | [**Currency**](Currency.md) |  | 
**dispute_stage** | [**DisputeStage**](DisputeStage.md) |  | 
**dispute_status** | [**DisputeStatus**](DisputeStatus.md) |  | 
**connector** | **str** | connector to which dispute is associated with | 
**connector_status** | **str** | Status of the dispute sent by connector | 
**connector_dispute_id** | **str** | Dispute id sent by connector | 
**connector_reason** | **str** | Reason of dispute sent by connector | [optional] 
**connector_reason_code** | **str** | Reason code of dispute sent by connector | [optional] 
**challenge_required_by** | **datetime** | Evidence deadline of dispute sent by connector | [optional] 
**connector_created_at** | **datetime** | Dispute created time sent by connector | [optional] 
**connector_updated_at** | **datetime** | Dispute updated time sent by connector | [optional] 
**created_at** | **datetime** | Time at which dispute is received | 
**profile_id** | **str** | The &#x60;profile_id&#x60; associated with the dispute | [optional] 
**merchant_connector_id** | **str** | The &#x60;merchant_connector_id&#x60; of the connector / processor through which the dispute was processed | [optional] 

## Example

```python
from hyperswitch.models.dispute_response import DisputeResponse

# TODO update the JSON string below
json = "{}"
# create an instance of DisputeResponse from a JSON string
dispute_response_instance = DisputeResponse.from_json(json)
# print the JSON string representation of the object
print(DisputeResponse.to_json())

# convert the object into a dict
dispute_response_dict = dispute_response_instance.to_dict()
# create an instance of DisputeResponse from a dict
dispute_response_from_dict = DisputeResponse.from_dict(dispute_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


