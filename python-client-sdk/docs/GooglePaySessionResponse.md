# GooglePaySessionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_info** | [**GpayMerchantInfo**](GpayMerchantInfo.md) |  | 
**shipping_address_required** | **bool** | Is shipping address required | 
**email_required** | **bool** | Is email required | 
**shipping_address_parameters** | [**GpayShippingAddressParameters**](GpayShippingAddressParameters.md) |  | 
**allowed_payment_methods** | [**List[GpayAllowedPaymentMethods]**](GpayAllowedPaymentMethods.md) | List of the allowed payment meythods | 
**transaction_info** | [**GpayTransactionInfo**](GpayTransactionInfo.md) |  | 
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**connector** | **str** | The name of the connector | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 
**secrets** | [**SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md) |  | [optional] 

## Example

```python
from hyperswitch.models.google_pay_session_response import GooglePaySessionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of GooglePaySessionResponse from a JSON string
google_pay_session_response_instance = GooglePaySessionResponse.from_json(json)
# print the JSON string representation of the object
print(GooglePaySessionResponse.to_json())

# convert the object into a dict
google_pay_session_response_dict = google_pay_session_response_instance.to_dict()
# create an instance of GooglePaySessionResponse from a dict
google_pay_session_response_from_dict = GooglePaySessionResponse.from_dict(google_pay_session_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


