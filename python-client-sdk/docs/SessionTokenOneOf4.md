# SessionTokenOneOf4


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
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of4 import SessionTokenOneOf4

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf4 from a JSON string
session_token_one_of4_instance = SessionTokenOneOf4.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf4.to_json())

# convert the object into a dict
session_token_one_of4_dict = session_token_one_of4_instance.to_dict()
# create an instance of SessionTokenOneOf4 from a dict
session_token_one_of4_from_dict = SessionTokenOneOf4.from_dict(session_token_one_of4_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


