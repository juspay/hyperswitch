# PayoutListResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**size** | **int** | The number of payouts included in the list | 
**data** | [**List[PayoutCreateResponse]**](PayoutCreateResponse.md) | The list of payouts response objects | 
**total_count** | **int** | The total number of available payouts for given constraints | [optional] 

## Example

```python
from hyperswitch.models.payout_list_response import PayoutListResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutListResponse from a JSON string
payout_list_response_instance = PayoutListResponse.from_json(json)
# print the JSON string representation of the object
print(PayoutListResponse.to_json())

# convert the object into a dict
payout_list_response_dict = payout_list_response_instance.to_dict()
# create an instance of PayoutListResponse from a dict
payout_list_response_from_dict = PayoutListResponse.from_dict(payout_list_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


