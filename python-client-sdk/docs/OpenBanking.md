# OpenBanking


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking** | [**OpenBankingData**](OpenBankingData.md) |  | 

## Example

```python
from hyperswitch.models.open_banking import OpenBanking

# TODO update the JSON string below
json = "{}"
# create an instance of OpenBanking from a JSON string
open_banking_instance = OpenBanking.from_json(json)
# print the JSON string representation of the object
print(OpenBanking.to_json())

# convert the object into a dict
open_banking_dict = open_banking_instance.to_dict()
# create an instance of OpenBanking from a dict
open_banking_from_dict = OpenBanking.from_dict(open_banking_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


