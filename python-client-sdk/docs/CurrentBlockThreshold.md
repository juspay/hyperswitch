# CurrentBlockThreshold


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**duration_in_mins** | **int** |  | [optional] 
**max_total_count** | **int** |  | [optional] 

## Example

```python
from hyperswitch.models.current_block_threshold import CurrentBlockThreshold

# TODO update the JSON string below
json = "{}"
# create an instance of CurrentBlockThreshold from a JSON string
current_block_threshold_instance = CurrentBlockThreshold.from_json(json)
# print the JSON string representation of the object
print(CurrentBlockThreshold.to_json())

# convert the object into a dict
current_block_threshold_dict = current_block_threshold_instance.to_dict()
# create an instance of CurrentBlockThreshold from a dict
current_block_threshold_from_dict = CurrentBlockThreshold.from_dict(current_block_threshold_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


