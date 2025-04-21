# SessionTokenOneOf1


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
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of1 import SessionTokenOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf1 from a JSON string
session_token_one_of1_instance = SessionTokenOneOf1.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf1.to_json())

# convert the object into a dict
session_token_one_of1_dict = session_token_one_of1_instance.to_dict()
# create an instance of SessionTokenOneOf1 from a dict
session_token_one_of1_from_dict = SessionTokenOneOf1.from_dict(session_token_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


