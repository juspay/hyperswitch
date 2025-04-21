# PayoutAttemptResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**attempt_id** | **str** | Unique identifier for the attempt | 
**status** | [**PayoutStatus**](PayoutStatus.md) |  | 
**amount** | **int** | The payout attempt amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., | 
**currency** | [**Currency**](Currency.md) |  | [optional] 
**connector** | **str** | The connector used for the payout | [optional] 
**error_code** | **str** | Connector&#39;s error code in case of failures | [optional] 
**error_message** | **str** | Connector&#39;s error message in case of failures | [optional] 
**payment_method** | [**PayoutType**](PayoutType.md) |  | [optional] 
**payout_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | [optional] 
**connector_transaction_id** | **str** | A unique identifier for a payout provided by the connector | [optional] 
**cancellation_reason** | **str** | If the payout was cancelled the reason provided here | [optional] 
**unified_code** | **str** | (This field is not live yet) Error code unified across the connectors is received here in case of errors while calling the underlying connector | [optional] 
**unified_message** | **str** | (This field is not live yet) Error message unified across the connectors is received here in case of errors while calling the underlying connector | [optional] 

## Example

```python
from hyperswitch.models.payout_attempt_response import PayoutAttemptResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutAttemptResponse from a JSON string
payout_attempt_response_instance = PayoutAttemptResponse.from_json(json)
# print the JSON string representation of the object
print(PayoutAttemptResponse.to_json())

# convert the object into a dict
payout_attempt_response_dict = payout_attempt_response_instance.to_dict()
# create an instance of PayoutAttemptResponse from a dict
payout_attempt_response_from_dict = PayoutAttemptResponse.from_dict(payout_attempt_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


