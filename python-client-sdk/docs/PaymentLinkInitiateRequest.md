# PaymentLinkInitiateRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** |  | 
**payment_id** | **str** |  | 

## Example

```python
from hyperswitch.models.payment_link_initiate_request import PaymentLinkInitiateRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentLinkInitiateRequest from a JSON string
payment_link_initiate_request_instance = PaymentLinkInitiateRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentLinkInitiateRequest.to_json())

# convert the object into a dict
payment_link_initiate_request_dict = payment_link_initiate_request_instance.to_dict()
# create an instance of PaymentLinkInitiateRequest from a dict
payment_link_initiate_request_from_dict = PaymentLinkInitiateRequest.from_dict(payment_link_initiate_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


