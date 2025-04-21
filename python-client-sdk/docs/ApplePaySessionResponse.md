# ApplePaySessionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**secrets** | [**SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md) |  | 
**epoch_timestamp** | **int** | Timestamp at which session is requested | 
**expires_at** | **int** | Timestamp at which session expires | 
**merchant_session_identifier** | **str** | The identifier for the merchant session | 
**nonce** | **str** | Apple pay generated unique ID (UUID) value | 
**merchant_identifier** | **str** | The identifier for the merchant | 
**domain_name** | **str** | The domain name of the merchant which is registered in Apple Pay | 
**display_name** | **str** | The name to be displayed on Apple Pay button | 
**signature** | **str** | A string which represents the properties of a payment | 
**operational_analytics_identifier** | **str** | The identifier for the operational analytics | 
**retries** | **int** | The number of retries to get the session response | 
**psp_id** | **str** | The identifier for the connector transaction | 

## Example

```python
from hyperswitch.models.apple_pay_session_response import ApplePaySessionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ApplePaySessionResponse from a JSON string
apple_pay_session_response_instance = ApplePaySessionResponse.from_json(json)
# print the JSON string representation of the object
print(ApplePaySessionResponse.to_json())

# convert the object into a dict
apple_pay_session_response_dict = apple_pay_session_response_instance.to_dict()
# create an instance of ApplePaySessionResponse from a dict
apple_pay_session_response_from_dict = ApplePaySessionResponse.from_dict(apple_pay_session_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


