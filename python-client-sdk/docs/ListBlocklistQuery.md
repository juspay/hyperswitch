# ListBlocklistQuery


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**data_kind** | [**BlocklistDataKind**](BlocklistDataKind.md) |  | 
**limit** | **int** |  | [optional] 
**offset** | **int** |  | [optional] 

## Example

```python
from hyperswitch.models.list_blocklist_query import ListBlocklistQuery

# TODO update the JSON string below
json = "{}"
# create an instance of ListBlocklistQuery from a JSON string
list_blocklist_query_instance = ListBlocklistQuery.from_json(json)
# print the JSON string representation of the object
print(ListBlocklistQuery.to_json())

# convert the object into a dict
list_blocklist_query_dict = list_blocklist_query_instance.to_dict()
# create an instance of ListBlocklistQuery from a dict
list_blocklist_query_from_dict = ListBlocklistQuery.from_dict(list_blocklist_query_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


