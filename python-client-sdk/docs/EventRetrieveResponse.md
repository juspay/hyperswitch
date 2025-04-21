# EventRetrieveResponse

The response body for retrieving an event.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**event_id** | **str** | The identifier for the Event. | 
**merchant_id** | **str** | The identifier for the Merchant Account. | 
**profile_id** | **str** | The identifier for the Business Profile. | 
**object_id** | **str** | The identifier for the object (Payment Intent ID, Refund ID, etc.) | 
**event_type** | [**EventType**](EventType.md) |  | 
**event_class** | [**EventClass**](EventClass.md) |  | 
**is_delivery_successful** | **bool** | Indicates whether the webhook was ultimately delivered or not. | [optional] 
**initial_attempt_id** | **str** | The identifier for the initial delivery attempt. This will be the same as &#x60;event_id&#x60; for the initial delivery attempt. | 
**created** | **datetime** | Time at which the event was created. | 
**request** | [**OutgoingWebhookRequestContent**](OutgoingWebhookRequestContent.md) |  | 
**response** | [**OutgoingWebhookResponseContent**](OutgoingWebhookResponseContent.md) |  | 
**delivery_attempt** | [**WebhookDeliveryAttempt**](WebhookDeliveryAttempt.md) |  | [optional] 

## Example

```python
from hyperswitch.models.event_retrieve_response import EventRetrieveResponse

# TODO update the JSON string below
json = "{}"
# create an instance of EventRetrieveResponse from a JSON string
event_retrieve_response_instance = EventRetrieveResponse.from_json(json)
# print the JSON string representation of the object
print(EventRetrieveResponse.to_json())

# convert the object into a dict
event_retrieve_response_dict = event_retrieve_response_instance.to_dict()
# create an instance of EventRetrieveResponse from a dict
event_retrieve_response_from_dict = EventRetrieveResponse.from_dict(event_retrieve_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


