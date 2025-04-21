# PaymentsCaptureRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The unique identifier for the merchant | [optional] 
**amount_to_capture** | **int** | The Amount to be captured/ debited from the user&#39;s payment method. If not passed the full amount will be captured. | 
**refund_uncaptured_amount** | **bool** | Decider to refund the uncaptured amount | [optional] 
**statement_descriptor_suffix** | **str** | Provides information about a card payment that customers see on their statements. | [optional] 
**statement_descriptor_prefix** | **str** | Concatenated with the statement descriptor suffix that’s set on the account to form the complete statement descriptor. | [optional] 
**merchant_connector_details** | [**MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payments_capture_request import PaymentsCaptureRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsCaptureRequest from a JSON string
payments_capture_request_instance = PaymentsCaptureRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsCaptureRequest.to_json())

# convert the object into a dict
payments_capture_request_dict = payments_capture_request_instance.to_dict()
# create an instance of PaymentsCaptureRequest from a dict
payments_capture_request_from_dict = PaymentsCaptureRequest.from_dict(payments_capture_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


