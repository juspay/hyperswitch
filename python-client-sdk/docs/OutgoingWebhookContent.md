# OutgoingWebhookContent


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**object** | [**PayoutCreateResponse**](PayoutCreateResponse.md) |  | 

## Example

```python
from hyperswitch.models.outgoing_webhook_content import OutgoingWebhookContent

# TODO update the JSON string below
json = "{}"
# create an instance of OutgoingWebhookContent from a JSON string
outgoing_webhook_content_instance = OutgoingWebhookContent.from_json(json)
# print the JSON string representation of the object
print(OutgoingWebhookContent.to_json())

# convert the object into a dict
outgoing_webhook_content_dict = outgoing_webhook_content_instance.to_dict()
# create an instance of OutgoingWebhookContent from a dict
outgoing_webhook_content_from_dict = OutgoingWebhookContent.from_dict(outgoing_webhook_content_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


