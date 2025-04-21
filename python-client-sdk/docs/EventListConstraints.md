# EventListConstraints

The constraints to apply when filtering events.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**created_after** | **datetime** | Filter events created after the specified time. | [optional] 
**created_before** | **datetime** | Filter events created before the specified time. | [optional] 
**limit** | **int** | Include at most the specified number of events. | [optional] 
**offset** | **int** | Include events after the specified offset. | [optional] 
**object_id** | **str** | Filter all events associated with the specified object identifier (Payment Intent ID, Refund ID, etc.) | [optional] 
**profile_id** | **str** | Filter all events associated with the specified business profile ID. | [optional] 
**event_classes** | [**List[EventClass]**](EventClass.md) | Filter events by their class. | [optional] 
**event_types** | [**List[EventType]**](EventType.md) | Filter events by their type. | [optional] 
**is_delivered** | **bool** | Filter all events by &#x60;is_overall_delivery_successful&#x60; field of the event. | [optional] 

## Example

```python
from hyperswitch.models.event_list_constraints import EventListConstraints

# TODO update the JSON string below
json = "{}"
# create an instance of EventListConstraints from a JSON string
event_list_constraints_instance = EventListConstraints.from_json(json)
# print the JSON string representation of the object
print(EventListConstraints.to_json())

# convert the object into a dict
event_list_constraints_dict = event_list_constraints_instance.to_dict()
# create an instance of EventListConstraints from a dict
event_list_constraints_from_dict = EventListConstraints.from_dict(event_list_constraints_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


