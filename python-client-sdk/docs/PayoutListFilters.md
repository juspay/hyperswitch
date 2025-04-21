# PayoutListFilters


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | [**List[PayoutConnectors]**](PayoutConnectors.md) | The list of available connector filters | 
**currency** | [**List[Currency]**](Currency.md) | The list of available currency filters | 
**status** | [**List[PayoutStatus]**](PayoutStatus.md) | The list of available payout status filters | 
**payout_method** | [**List[PayoutType]**](PayoutType.md) | The list of available payout method filters | 

## Example

```python
from hyperswitch.models.payout_list_filters import PayoutListFilters

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutListFilters from a JSON string
payout_list_filters_instance = PayoutListFilters.from_json(json)
# print the JSON string representation of the object
print(PayoutListFilters.to_json())

# convert the object into a dict
payout_list_filters_dict = payout_list_filters_instance.to_dict()
# create an instance of PayoutListFilters from a dict
payout_list_filters_from_dict = PayoutListFilters.from_dict(payout_list_filters_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


