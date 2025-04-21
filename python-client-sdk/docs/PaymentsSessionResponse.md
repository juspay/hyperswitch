# PaymentsSessionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **str** | The identifier for the payment | 
**client_secret** | **str** | This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK | 
**session_token** | [**List[SessionToken]**](SessionToken.md) | The list of session token object | 

## Example

```python
from hyperswitch.models.payments_session_response import PaymentsSessionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsSessionResponse from a JSON string
payments_session_response_instance = PaymentsSessionResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentsSessionResponse.to_json())

# convert the object into a dict
payments_session_response_dict = payments_session_response_instance.to_dict()
# create an instance of PaymentsSessionResponse from a dict
payments_session_response_from_dict = PaymentsSessionResponse.from_dict(payments_session_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


