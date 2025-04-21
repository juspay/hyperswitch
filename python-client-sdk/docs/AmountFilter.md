# AmountFilter


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**start_amount** | **int** | The start amount to filter list of transactions which are greater than or equal to the start amount | [optional] 
**end_amount** | **int** | The end amount to filter list of transactions which are less than or equal to the end amount | [optional] 

## Example

```python
from hyperswitch.models.amount_filter import AmountFilter

# TODO update the JSON string below
json = "{}"
# create an instance of AmountFilter from a JSON string
amount_filter_instance = AmountFilter.from_json(json)
# print the JSON string representation of the object
print(AmountFilter.to_json())

# convert the object into a dict
amount_filter_dict = amount_filter_instance.to_dict()
# create an instance of AmountFilter from a dict
amount_filter_from_dict = AmountFilter.from_dict(amount_filter_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


