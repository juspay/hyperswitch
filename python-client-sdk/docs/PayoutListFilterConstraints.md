# PayoutListFilterConstraints


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**start_time** | **datetime** | The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed | 
**end_time** | **datetime** | The end time to filter payments list or to get list of filters. If not passed the default time is now | [optional] 
**payout_id** | **str** | The identifier for payout | [optional] 
**profile_id** | **str** | The identifier for business profile | [optional] 
**customer_id** | **str** | The identifier for customer | [optional] 
**limit** | **int** | The limit on the number of objects. The default limit is 10 and max limit is 20 | [optional] 
**offset** | **int** | The starting point within a list of objects | [optional] 
**connector** | [**List[PayoutConnectors]**](PayoutConnectors.md) | The list of connectors to filter payouts list | [optional] 
**currency** | [**Currency**](Currency.md) |  | 
**status** | [**List[PayoutStatus]**](PayoutStatus.md) | The list of payout status to filter payouts list | [optional] 
**payout_method** | [**List[PayoutType]**](PayoutType.md) | The list of payout methods to filter payouts list | [optional] 
**entity_type** | [**PayoutEntityType**](PayoutEntityType.md) |  | 

## Example

```python
from hyperswitch.models.payout_list_filter_constraints import PayoutListFilterConstraints

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutListFilterConstraints from a JSON string
payout_list_filter_constraints_instance = PayoutListFilterConstraints.from_json(json)
# print the JSON string representation of the object
print(PayoutListFilterConstraints.to_json())

# convert the object into a dict
payout_list_filter_constraints_dict = payout_list_filter_constraints_instance.to_dict()
# create an instance of PayoutListFilterConstraints from a dict
payout_list_filter_constraints_from_dict = PayoutListFilterConstraints.from_dict(payout_list_filter_constraints_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


