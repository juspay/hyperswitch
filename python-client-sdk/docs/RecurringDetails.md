# RecurringDetails

Details required for recurring payment

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | [**NetworkTransactionIdAndCardDetails**](NetworkTransactionIdAndCardDetails.md) |  | 

## Example

```python
from hyperswitch.models.recurring_details import RecurringDetails

# TODO update the JSON string below
json = "{}"
# create an instance of RecurringDetails from a JSON string
recurring_details_instance = RecurringDetails.from_json(json)
# print the JSON string representation of the object
print(RecurringDetails.to_json())

# convert the object into a dict
recurring_details_dict = recurring_details_instance.to_dict()
# create an instance of RecurringDetails from a dict
recurring_details_from_dict = RecurringDetails.from_dict(recurring_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


