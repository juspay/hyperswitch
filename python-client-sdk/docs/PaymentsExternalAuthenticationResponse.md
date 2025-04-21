# PaymentsExternalAuthenticationResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**trans_status** | [**TransactionStatus**](TransactionStatus.md) |  | 
**acs_url** | **str** | Access Server URL to be used for challenge submission | [optional] 
**challenge_request** | **str** | Challenge request which should be sent to acs_url | [optional] 
**acs_reference_number** | **str** | Unique identifier assigned by the EMVCo(Europay, Mastercard and Visa) | [optional] 
**acs_trans_id** | **str** | Unique identifier assigned by the ACS to identify a single transaction | [optional] 
**three_dsserver_trans_id** | **str** | Unique identifier assigned by the 3DS Server to identify a single transaction | [optional] 
**acs_signed_content** | **str** | Contains the JWS object created by the ACS for the ARes(Authentication Response) message | [optional] 
**three_ds_requestor_url** | **str** | Three DS Requestor URL | 
**three_ds_requestor_app_url** | **str** | Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred | [optional] 

## Example

```python
from hyperswitch.models.payments_external_authentication_response import PaymentsExternalAuthenticationResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsExternalAuthenticationResponse from a JSON string
payments_external_authentication_response_instance = PaymentsExternalAuthenticationResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentsExternalAuthenticationResponse.to_json())

# convert the object into a dict
payments_external_authentication_response_dict = payments_external_authentication_response_instance.to_dict()
# create an instance of PaymentsExternalAuthenticationResponse from a dict
payments_external_authentication_response_from_dict = PaymentsExternalAuthenticationResponse.from_dict(payments_external_authentication_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


