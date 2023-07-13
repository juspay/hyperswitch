# PaymentListConstraints

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_id** | Option<**String**> | The identifier for customer | [optional]
**starting_after** | Option<**String**> | A cursor for use in pagination, fetch the next list after some object | [optional]
**ending_before** | Option<**String**> | A cursor for use in pagination, fetch the previous list before some object | [optional]
**limit** | Option<**i64**> | limit on the number of objects to return | [optional][default to 10]
**created** | Option<**String**> | The time at which payment is created | [optional]
**created_period_lt** | Option<**String**> | Time less than the payment created time | [optional]
**created_period_gt** | Option<**String**> | Time greater than the payment created time | [optional]
**created_period_lte** | Option<**String**> | Time less than or equals to the payment created time | [optional]
**created_period_gte** | Option<**String**> | Time greater than or equals to the payment created time | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


