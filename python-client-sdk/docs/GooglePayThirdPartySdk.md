# GooglePayThirdPartySdk


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**connector** | **str** | The name of the connector | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 

## Example

```python
from hyperswitch.models.google_pay_third_party_sdk import GooglePayThirdPartySdk

# TODO update the JSON string below
json = "{}"
# create an instance of GooglePayThirdPartySdk from a JSON string
google_pay_third_party_sdk_instance = GooglePayThirdPartySdk.from_json(json)
# print the JSON string representation of the object
print(GooglePayThirdPartySdk.to_json())

# convert the object into a dict
google_pay_third_party_sdk_dict = google_pay_third_party_sdk_instance.to_dict()
# create an instance of GooglePayThirdPartySdk from a dict
google_pay_third_party_sdk_from_dict = GooglePayThirdPartySdk.from_dict(google_pay_third_party_sdk_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


