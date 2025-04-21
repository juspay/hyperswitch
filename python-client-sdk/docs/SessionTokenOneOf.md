# SessionTokenOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**connector** | **str** | The name of the connector | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 
**merchant_info** | [**GpayMerchantInfo**](GpayMerchantInfo.md) |  | 
**shipping_address_required** | **bool** | Is shipping address required | 
**email_required** | **bool** | Is email required | 
**shipping_address_parameters** | [**GpayShippingAddressParameters**](GpayShippingAddressParameters.md) |  | 
**allowed_payment_methods** | [**List[GpayAllowedPaymentMethods]**](GpayAllowedPaymentMethods.md) | List of the allowed payment meythods | 
**transaction_info** | [**GpayTransactionInfo**](GpayTransactionInfo.md) |  | 
**secrets** | [**SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md) |  | [optional] 
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of import SessionTokenOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf from a JSON string
session_token_one_of_instance = SessionTokenOneOf.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf.to_json())

# convert the object into a dict
session_token_one_of_dict = session_token_one_of_instance.to_dict()
# create an instance of SessionTokenOneOf from a dict
session_token_one_of_from_dict = SessionTokenOneOf.from_dict(session_token_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


