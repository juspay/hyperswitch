# PaymentMethodCollectLinkResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**logo** | **str** | Merchant&#39;s display logo | [optional] 
**merchant_name** | **str** | Custom merchant name for the link | [optional] 
**theme** | **str** | Primary color to be used in the form represented in hex format | [optional] 
**pm_collect_link_id** | **str** | The unique identifier for the collect link. | 
**customer_id** | **str** | The unique identifier of the customer. | 
**expiry** | **datetime** | Time when this link will be expired in ISO8601 format | 
**link** | **str** | URL to the form&#39;s link generated for collecting payment method details. | 
**return_url** | **str** | Redirect to this URL post completion | [optional] 
**enabled_payment_methods** | [**List[EnabledPaymentMethod]**](EnabledPaymentMethod.md) | List of payment methods shown on collect UI | [optional] 

## Example

```python
from hyperswitch.models.payment_method_collect_link_response import PaymentMethodCollectLinkResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodCollectLinkResponse from a JSON string
payment_method_collect_link_response_instance = PaymentMethodCollectLinkResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodCollectLinkResponse.to_json())

# convert the object into a dict
payment_method_collect_link_response_dict = payment_method_collect_link_response_instance.to_dict()
# create an instance of PaymentMethodCollectLinkResponse from a dict
payment_method_collect_link_response_from_dict = PaymentMethodCollectLinkResponse.from_dict(payment_method_collect_link_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


