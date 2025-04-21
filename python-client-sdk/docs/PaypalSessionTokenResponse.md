# PaypalSessionTokenResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | **str** | Name of the connector | 
**session_token** | **str** | The session token for PayPal | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 

## Example

```python
from hyperswitch.models.paypal_session_token_response import PaypalSessionTokenResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaypalSessionTokenResponse from a JSON string
paypal_session_token_response_instance = PaypalSessionTokenResponse.from_json(json)
# print the JSON string representation of the object
print(PaypalSessionTokenResponse.to_json())

# convert the object into a dict
paypal_session_token_response_dict = paypal_session_token_response_instance.to_dict()
# create an instance of PaypalSessionTokenResponse from a dict
paypal_session_token_response_from_dict = PaypalSessionTokenResponse.from_dict(paypal_session_token_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


