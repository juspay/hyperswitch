# OutgoingWebhook


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The merchant id of the merchant | 
**event_id** | **str** | The unique event id for each webhook | 
**event_type** | [**EventType**](EventType.md) |  | 
**content** | [**OutgoingWebhookContent**](OutgoingWebhookContent.md) |  | 
**timestamp** | **datetime** | The time at which webhook was sent | [optional] 

## Example

```python
from hyperswitch.models.outgoing_webhook import OutgoingWebhook

# TODO update the JSON string below
json = "{}"
# create an instance of OutgoingWebhook from a JSON string
outgoing_webhook_instance = OutgoingWebhook.from_json(json)
# print the JSON string representation of the object
print(OutgoingWebhook.to_json())

# convert the object into a dict
outgoing_webhook_dict = outgoing_webhook_instance.to_dict()
# create an instance of OutgoingWebhook from a dict
outgoing_webhook_from_dict = OutgoingWebhook.from_dict(outgoing_webhook_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


