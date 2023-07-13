# DisputeResponsePaymentsRetrieve

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**dispute_id** | **String** | The identifier for dispute | 
**dispute_stage** | [**crate::models::DisputeStage**](DisputeStage.md) |  | 
**dispute_status** | [**crate::models::DisputeStatus**](DisputeStatus.md) |  | 
**connector_status** | **String** | Status of the dispute sent by connector | 
**connector_dispute_id** | **String** | Dispute id sent by connector | 
**connector_reason** | Option<**String**> | Reason of dispute sent by connector | [optional]
**connector_reason_code** | Option<**String**> | Reason code of dispute sent by connector | [optional]
**challenge_required_by** | Option<**String**> | Evidence deadline of dispute sent by connector | [optional]
**connector_created_at** | Option<**String**> | Dispute created time sent by connector | [optional]
**connector_updated_at** | Option<**String**> | Dispute updated time sent by connector | [optional]
**created_at** | **String** | Time at which dispute is received | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


