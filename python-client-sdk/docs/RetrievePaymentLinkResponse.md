# RetrievePaymentLinkResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_link_id** | **str** | Identifier for Payment Link | 
**merchant_id** | **str** | Identifier for Merchant | 
**link_to_pay** | **str** | Open payment link (without any security checks and listing SPMs) | 
**amount** | **int** | The payment amount. Amount for the payment in the lowest denomination of the currency | 
**created_at** | **datetime** | Date and time of Payment Link creation | 
**expiry** | **datetime** | Date and time of Expiration for Payment Link | [optional] 
**description** | **str** | Description for Payment Link | [optional] 
**status** | [**PaymentLinkStatus**](PaymentLinkStatus.md) |  | 
**currency** | [**Currency**](Currency.md) |  | [optional] 
**secure_link** | **str** | Secure payment link (with security checks and listing saved payment methods) | [optional] 

## Example

```python
from hyperswitch.models.retrieve_payment_link_response import RetrievePaymentLinkResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RetrievePaymentLinkResponse from a JSON string
retrieve_payment_link_response_instance = RetrievePaymentLinkResponse.from_json(json)
# print the JSON string representation of the object
print(RetrievePaymentLinkResponse.to_json())

# convert the object into a dict
retrieve_payment_link_response_dict = retrieve_payment_link_response_instance.to_dict()
# create an instance of RetrievePaymentLinkResponse from a dict
retrieve_payment_link_response_from_dict = RetrievePaymentLinkResponse.from_dict(retrieve_payment_link_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


