# PaymentMethodCollectLinkRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**logo** | **str** | Merchant&#39;s display logo | [optional] 
**merchant_name** | **str** | Custom merchant name for the link | [optional] 
**theme** | **str** | Primary color to be used in the form represented in hex format | [optional] 
**pm_collect_link_id** | **str** | The unique identifier for the collect link. | [optional] 
**customer_id** | **str** | The unique identifier of the customer. | 
**session_expiry** | **int** | Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins | [optional] 
**return_url** | **str** | Redirect to this URL post completion | [optional] 
**enabled_payment_methods** | [**List[EnabledPaymentMethod]**](EnabledPaymentMethod.md) | List of payment methods shown on collect UI | [optional] 

## Example

```python
from hyperswitch.models.payment_method_collect_link_request import PaymentMethodCollectLinkRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodCollectLinkRequest from a JSON string
payment_method_collect_link_request_instance = PaymentMethodCollectLinkRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodCollectLinkRequest.to_json())

# convert the object into a dict
payment_method_collect_link_request_dict = payment_method_collect_link_request_instance.to_dict()
# create an instance of PaymentMethodCollectLinkRequest from a dict
payment_method_collect_link_request_from_dict = PaymentMethodCollectLinkRequest.from_dict(payment_method_collect_link_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


