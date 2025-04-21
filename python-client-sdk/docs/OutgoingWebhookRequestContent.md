# OutgoingWebhookRequestContent

The request information (headers and body) sent in the webhook.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**body** | **str** | The request body sent in the webhook. | 
**headers** | **List[List[OutgoingWebhookRequestContentHeadersInnerInner]]** | The request headers sent in the webhook. | 

## Example

```python
from hyperswitch.models.outgoing_webhook_request_content import OutgoingWebhookRequestContent

# TODO update the JSON string below
json = "{}"
# create an instance of OutgoingWebhookRequestContent from a JSON string
outgoing_webhook_request_content_instance = OutgoingWebhookRequestContent.from_json(json)
# print the JSON string representation of the object
print(OutgoingWebhookRequestContent.to_json())

# convert the object into a dict
outgoing_webhook_request_content_dict = outgoing_webhook_request_content_instance.to_dict()
# create an instance of OutgoingWebhookRequestContent from a dict
outgoing_webhook_request_content_from_dict = OutgoingWebhookRequestContent.from_dict(outgoing_webhook_request_content_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


