# ReceiverDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount_received** | **int** | The amount received by receiver | 
**amount_charged** | **int** | The amount charged by ACH | [optional] 
**amount_remaining** | **int** | The amount remaining to be sent via ACH | [optional] 

## Example

```python
from hyperswitch.models.receiver_details import ReceiverDetails

# TODO update the JSON string below
json = "{}"
# create an instance of ReceiverDetails from a JSON string
receiver_details_instance = ReceiverDetails.from_json(json)
# print the JSON string representation of the object
print(ReceiverDetails.to_json())

# convert the object into a dict
receiver_details_dict = receiver_details_instance.to_dict()
# create an instance of ReceiverDetails from a dict
receiver_details_from_dict = ReceiverDetails.from_dict(receiver_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


