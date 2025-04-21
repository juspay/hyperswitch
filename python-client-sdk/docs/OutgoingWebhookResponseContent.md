# OutgoingWebhookResponseContent

The response information (headers, body and status code) received for the webhook sent.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**body** | **str** | The response body received for the webhook sent. | [optional] 
**headers** | **List[List[OutgoingWebhookRequestContentHeadersInnerInner]]** | The response headers received for the webhook sent. | [optional] 
**status_code** | **int** | The HTTP status code for the webhook sent. | [optional] 
**error_message** | **str** | Error message in case any error occurred when trying to deliver the webhook. | [optional] 

## Example

```python
from hyperswitch.models.outgoing_webhook_response_content import OutgoingWebhookResponseContent

# TODO update the JSON string below
json = "{}"
# create an instance of OutgoingWebhookResponseContent from a JSON string
outgoing_webhook_response_content_instance = OutgoingWebhookResponseContent.from_json(json)
# print the JSON string representation of the object
print(OutgoingWebhookResponseContent.to_json())

# convert the object into a dict
outgoing_webhook_response_content_dict = outgoing_webhook_response_content_instance.to_dict()
# create an instance of OutgoingWebhookResponseContent from a dict
outgoing_webhook_response_content_from_dict = OutgoingWebhookResponseContent.from_dict(outgoing_webhook_response_content_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


