# RetrievePaymentLinkRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**client_secret** | **str** | It&#39;s a token used for client side verification. | [optional] 

## Example

```python
from hyperswitch.models.retrieve_payment_link_request import RetrievePaymentLinkRequest

# TODO update the JSON string below
json = "{}"
# create an instance of RetrievePaymentLinkRequest from a JSON string
retrieve_payment_link_request_instance = RetrievePaymentLinkRequest.from_json(json)
# print the JSON string representation of the object
print(RetrievePaymentLinkRequest.to_json())

# convert the object into a dict
retrieve_payment_link_request_dict = retrieve_payment_link_request_instance.to_dict()
# create an instance of RetrievePaymentLinkRequest from a dict
retrieve_payment_link_request_from_dict = RetrievePaymentLinkRequest.from_dict(retrieve_payment_link_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


