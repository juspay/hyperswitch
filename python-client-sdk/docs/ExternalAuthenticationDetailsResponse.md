# ExternalAuthenticationDetailsResponse

Details of external authentication

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authentication_flow** | [**DecoupledAuthenticationType**](DecoupledAuthenticationType.md) |  | [optional] 
**electronic_commerce_indicator** | **str** | Electronic Commerce Indicator (eci) | [optional] 
**status** | [**AuthenticationStatus**](AuthenticationStatus.md) |  | 
**ds_transaction_id** | **str** | DS Transaction ID | [optional] 
**version** | **str** | Message Version | [optional] 
**error_code** | **str** | Error Code | [optional] 
**error_message** | **str** | Error Message | [optional] 

## Example

```python
from hyperswitch.models.external_authentication_details_response import ExternalAuthenticationDetailsResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ExternalAuthenticationDetailsResponse from a JSON string
external_authentication_details_response_instance = ExternalAuthenticationDetailsResponse.from_json(json)
# print the JSON string representation of the object
print(ExternalAuthenticationDetailsResponse.to_json())

# convert the object into a dict
external_authentication_details_response_dict = external_authentication_details_response_instance.to_dict()
# create an instance of ExternalAuthenticationDetailsResponse from a dict
external_authentication_details_response_from_dict = ExternalAuthenticationDetailsResponse.from_dict(external_authentication_details_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


