# RefundResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**refund_id** | **str** | Unique Identifier for the refund | 
**payment_id** | **str** | The payment id against which refund is initiated | 
**amount** | **int** | The refund amount, which should be less than or equal to the total payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc | 
**currency** | **str** | The three-letter ISO currency code | 
**status** | [**RefundStatus**](RefundStatus.md) |  | 
**reason** | **str** | An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive | [optional] 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object | [optional] 
**error_message** | **str** | The error message | [optional] 
**error_code** | **str** | The code for the error | [optional] 
**unified_code** | **str** | Error code unified across the connectors is received here if there was an error while calling connector | [optional] 
**unified_message** | **str** | Error message unified across the connectors is received here if there was an error while calling connector | [optional] 
**created_at** | **datetime** | The timestamp at which refund is created | [optional] 
**updated_at** | **datetime** | The timestamp at which refund is updated | [optional] 
**connector** | **str** | The connector used for the refund and the corresponding payment | 
**profile_id** | **str** | The id of business profile for this refund | [optional] 
**merchant_connector_id** | **str** | The merchant_connector_id of the processor through which this payment went through | [optional] 
**split_refunds** | [**SplitRefund**](SplitRefund.md) |  | [optional] 
**issuer_error_code** | **str** | Error code received from the issuer in case of failed refunds | [optional] 
**issuer_error_message** | **str** | Error message received from the issuer in case of failed refunds | [optional] 

## Example

```python
from hyperswitch.models.refund_response import RefundResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RefundResponse from a JSON string
refund_response_instance = RefundResponse.from_json(json)
# print the JSON string representation of the object
print(RefundResponse.to_json())

# convert the object into a dict
refund_response_dict = refund_response_instance.to_dict()
# create an instance of RefundResponse from a dict
refund_response_from_dict = RefundResponse.from_dict(refund_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


