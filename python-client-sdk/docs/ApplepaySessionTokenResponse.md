# ApplepaySessionTokenResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**session_token_data** | [**ApplePaySessionResponse**](ApplePaySessionResponse.md) |  | [optional] 
**payment_request_data** | [**ApplePayPaymentRequest**](ApplePayPaymentRequest.md) |  | [optional] 
**connector** | **str** | The session token is w.r.t this connector | 
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 
**connector_reference_id** | **str** | The connector transaction id | [optional] 
**connector_sdk_public_key** | **str** | The public key id is to invoke third party sdk | [optional] 
**connector_merchant_id** | **str** | The connector merchant id | [optional] 

## Example

```python
from hyperswitch.models.applepay_session_token_response import ApplepaySessionTokenResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ApplepaySessionTokenResponse from a JSON string
applepay_session_token_response_instance = ApplepaySessionTokenResponse.from_json(json)
# print the JSON string representation of the object
print(ApplepaySessionTokenResponse.to_json())

# convert the object into a dict
applepay_session_token_response_dict = applepay_session_token_response_instance.to_dict()
# create an instance of ApplepaySessionTokenResponse from a dict
applepay_session_token_response_from_dict = ApplepaySessionTokenResponse.from_dict(applepay_session_token_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


