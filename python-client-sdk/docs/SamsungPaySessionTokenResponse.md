# SamsungPaySessionTokenResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**version** | **str** | Samsung Pay API version | 
**service_id** | **str** | Samsung Pay service ID to which session call needs to be made | 
**order_number** | **str** | Order number of the transaction | 
**merchant** | [**SamsungPayMerchantPaymentInformation**](SamsungPayMerchantPaymentInformation.md) |  | 
**amount** | [**SamsungPayAmountDetails**](SamsungPayAmountDetails.md) |  | 
**protocol** | [**SamsungPayProtocolType**](SamsungPayProtocolType.md) |  | 
**allowed_brands** | **List[str]** | List of supported card brands | 
**billing_address_required** | **bool** | Is billing address required to be collected from wallet | 
**shipping_address_required** | **bool** | Is shipping address required to be collected from wallet | 

## Example

```python
from hyperswitch.models.samsung_pay_session_token_response import SamsungPaySessionTokenResponse

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPaySessionTokenResponse from a JSON string
samsung_pay_session_token_response_instance = SamsungPaySessionTokenResponse.from_json(json)
# print the JSON string representation of the object
print(SamsungPaySessionTokenResponse.to_json())

# convert the object into a dict
samsung_pay_session_token_response_dict = samsung_pay_session_token_response_instance.to_dict()
# create an instance of SamsungPaySessionTokenResponse from a dict
samsung_pay_session_token_response_from_dict = SamsungPaySessionTokenResponse.from_dict(samsung_pay_session_token_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


