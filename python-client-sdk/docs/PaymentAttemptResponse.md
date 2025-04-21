# PaymentAttemptResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**attempt_id** | **str** | Unique identifier for the attempt | 
**status** | [**AttemptStatus**](AttemptStatus.md) |  | 
**amount** | **int** | The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., | 
**order_tax_amount** | **int** | The payment attempt tax_amount. | [optional] 
**currency** | [**Currency**](Currency.md) |  | [optional] 
**connector** | **str** | The connector used for the payment | [optional] 
**error_message** | **str** | If there was an error while calling the connector, the error message is received here | [optional] 
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | [optional] 
**connector_transaction_id** | **str** | A unique identifier for a payment provided by the connector | [optional] 
**capture_method** | [**CaptureMethod**](CaptureMethod.md) |  | [optional] 
**authentication_type** | [**AuthenticationType**](AuthenticationType.md) |  | [optional] 
**created_at** | **datetime** | Time at which the payment attempt was created | 
**modified_at** | **datetime** | Time at which the payment attempt was last modified | 
**cancellation_reason** | **str** | If the payment was cancelled the reason will be provided here | [optional] 
**mandate_id** | **str** | A unique identifier to link the payment to a mandate, can be use instead of payment_method_data | [optional] 
**error_code** | **str** | If there was an error while calling the connectors the error code is received here | [optional] 
**payment_token** | **str** | Provide a reference to a stored payment method | [optional] 
**connector_metadata** | **object** | Additional data related to some connectors | [optional] 
**payment_experience** | [**PaymentExperience**](PaymentExperience.md) |  | [optional] 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | [optional] 
**reference_id** | **str** | Reference to the payment at connector side | [optional] 
**unified_code** | **str** | (This field is not live yet)Error code unified across the connectors is received here if there was an error while calling connector | [optional] 
**unified_message** | **str** | (This field is not live yet)Error message unified across the connectors is received here if there was an error while calling connector | [optional] 
**client_source** | **str** | Value passed in X-CLIENT-SOURCE header during payments confirm request by the client | [optional] 
**client_version** | **str** | Value passed in X-CLIENT-VERSION header during payments confirm request by the client | [optional] 

## Example

```python
from hyperswitch.models.payment_attempt_response import PaymentAttemptResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentAttemptResponse from a JSON string
payment_attempt_response_instance = PaymentAttemptResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentAttemptResponse.to_json())

# convert the object into a dict
payment_attempt_response_dict = payment_attempt_response_instance.to_dict()
# create an instance of PaymentAttemptResponse from a dict
payment_attempt_response_from_dict = PaymentAttemptResponse.from_dict(payment_attempt_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


