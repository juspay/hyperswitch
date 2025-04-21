# RewardData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The merchant ID with which we have to call the connector | 

## Example

```python
from hyperswitch.models.reward_data import RewardData

# TODO update the JSON string below
json = "{}"
# create an instance of RewardData from a JSON string
reward_data_instance = RewardData.from_json(json)
# print the JSON string representation of the object
print(RewardData.to_json())

# convert the object into a dict
reward_data_dict = reward_data_instance.to_dict()
# create an instance of RewardData from a dict
reward_data_from_dict = RewardData.from_dict(reward_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


