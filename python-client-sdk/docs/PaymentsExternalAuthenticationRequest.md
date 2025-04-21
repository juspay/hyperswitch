# PaymentsExternalAuthenticationRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**client_secret** | **str** | Client Secret | 
**sdk_information** | [**SdkInformation**](SdkInformation.md) |  | [optional] 
**device_channel** | [**DeviceChannel**](DeviceChannel.md) |  | 
**threeds_method_comp_ind** | [**ThreeDsCompletionIndicator**](ThreeDsCompletionIndicator.md) |  | 

## Example

```python
from hyperswitch.models.payments_external_authentication_request import PaymentsExternalAuthenticationRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsExternalAuthenticationRequest from a JSON string
payments_external_authentication_request_instance = PaymentsExternalAuthenticationRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsExternalAuthenticationRequest.to_json())

# convert the object into a dict
payments_external_authentication_request_dict = payments_external_authentication_request_instance.to_dict()
# create an instance of PaymentsExternalAuthenticationRequest from a dict
payments_external_authentication_request_from_dict = PaymentsExternalAuthenticationRequest.from_dict(payments_external_authentication_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


