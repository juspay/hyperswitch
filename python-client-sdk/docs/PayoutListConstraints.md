# PayoutListConstraints


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**start_time** | **datetime** | The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed | 
**end_time** | **datetime** | The end time to filter payments list or to get list of filters. If not passed the default time is now | [optional] 
**customer_id** | **str** | The identifier for customer | [optional] 
**starting_after** | **str** | A cursor for use in pagination, fetch the next list after some object | [optional] 
**ending_before** | **str** | A cursor for use in pagination, fetch the previous list before some object | [optional] 
**limit** | **int** | limit on the number of objects to return | [optional] [default to 10]
**created** | **datetime** | The time at which payout is created | [optional] 

## Example

```python
from hyperswitch.models.payout_list_constraints import PayoutListConstraints

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutListConstraints from a JSON string
payout_list_constraints_instance = PayoutListConstraints.from_json(json)
# print the JSON string representation of the object
print(PayoutListConstraints.to_json())

# convert the object into a dict
payout_list_constraints_dict = payout_list_constraints_instance.to_dict()
# create an instance of PayoutListConstraints from a dict
payout_list_constraints_from_dict = PayoutListConstraints.from_dict(payout_list_constraints_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


