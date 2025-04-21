# PaymentLinkResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**link** | **str** | URL for rendering the open payment link | 
**secure_link** | **str** | URL for rendering the secure payment link | [optional] 
**payment_link_id** | **str** | Identifier for the payment link | 

## Example

```python
from hyperswitch.models.payment_link_response import PaymentLinkResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentLinkResponse from a JSON string
payment_link_response_instance = PaymentLinkResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentLinkResponse.to_json())

# convert the object into a dict
payment_link_response_dict = payment_link_response_instance.to_dict()
# create an instance of PaymentLinkResponse from a dict
payment_link_response_from_dict = PaymentLinkResponse.from_dict(payment_link_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


