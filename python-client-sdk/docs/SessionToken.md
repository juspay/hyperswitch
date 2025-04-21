# SessionToken


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**connector** | **str** | The session token is w.r.t this connector | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 
**merchant_info** | [**GpayMerchantInfo**](GpayMerchantInfo.md) |  | 
**shipping_address_required** | **bool** | Is shipping address required to be collected from wallet | 
**email_required** | **bool** | Is email required | 
**shipping_address_parameters** | [**GpayShippingAddressParameters**](GpayShippingAddressParameters.md) |  | 
**allowed_payment_methods** | [**List[GpayAllowedPaymentMethods]**](GpayAllowedPaymentMethods.md) | List of the allowed payment meythods | 
**transaction_info** | [**GpayTransactionInfo**](GpayTransactionInfo.md) |  | 
**secrets** | [**SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md) |  | [optional] 
**wallet_name** | **str** |  | 
**version** | **str** | Samsung Pay API version | 
**service_id** | **str** | Samsung Pay service ID to which session call needs to be made | 
**order_number** | **str** | Order number of the transaction | 
**merchant** | [**SamsungPayMerchantPaymentInformation**](SamsungPayMerchantPaymentInformation.md) |  | 
**amount** | [**SamsungPayAmountDetails**](SamsungPayAmountDetails.md) |  | 
**protocol** | [**SamsungPayProtocolType**](SamsungPayProtocolType.md) |  | 
**allowed_brands** | **List[str]** | List of supported card brands | 
**billing_address_required** | **bool** | Is billing address required to be collected from wallet | 
**session_token** | **str** | The session token for PayPal | 
**session_id** | **str** | The identifier for the session | 
**session_token_data** | [**ApplePaySessionResponse**](ApplePaySessionResponse.md) |  | [optional] 
**payment_request_data** | [**ApplePayPaymentRequest**](ApplePayPaymentRequest.md) |  | [optional] 
**connector_reference_id** | **str** | The connector transaction id | [optional] 
**connector_sdk_public_key** | **str** | The public key id is to invoke third party sdk | [optional] 
**connector_merchant_id** | **str** | The connector merchant id | [optional] 
**open_banking_session_token** | **str** | The session token for OpenBanking Connectors | 
**client_id** | **str** | Paze Client ID | 
**client_name** | **str** | Client Name to be displayed on the Paze screen | 
**client_profile_id** | **str** | Paze Client Profile ID | 
**transaction_currency_code** | [**Currency**](Currency.md) |  | 
**transaction_amount** | **str** |  | 
**email_address** | **str** | Email Address | [optional] 
**dpa_id** | **str** |  | 
**dpa_name** | **str** |  | 
**locale** | **str** |  | 
**card_brands** | **List[str]** |  | 
**acquirer_bin** | **str** |  | 
**acquirer_merchant_id** | **str** |  | 
**merchant_category_code** | **str** |  | 
**merchant_country_code** | **str** |  | 
**phone_number** | **str** |  | [optional] 
**email** | **str** |  | [optional] 
**phone_country_code** | **str** |  | [optional] 
**provider** | [**CtpServiceProvider**](CtpServiceProvider.md) |  | [optional] 
**dpa_client_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.session_token import SessionToken

# TODO update the JSON string below
json = "{}"
# create an instance of SessionToken from a JSON string
session_token_instance = SessionToken.from_json(json)
# print the JSON string representation of the object
print(SessionToken.to_json())

# convert the object into a dict
session_token_dict = session_token_instance.to_dict()
# create an instance of SessionToken from a dict
session_token_from_dict = SessionToken.from_dict(session_token_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


