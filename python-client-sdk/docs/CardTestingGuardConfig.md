# CardTestingGuardConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_ip_blocking_status** | [**CardTestingGuardStatus**](CardTestingGuardStatus.md) |  | 
**card_ip_blocking_threshold** | **int** | Determines the unsuccessful payment threshold for Card IP Blocking for profile | 
**guest_user_card_blocking_status** | [**CardTestingGuardStatus**](CardTestingGuardStatus.md) |  | 
**guest_user_card_blocking_threshold** | **int** | Determines the unsuccessful payment threshold for Guest User Card Blocking for profile | 
**customer_id_blocking_status** | [**CardTestingGuardStatus**](CardTestingGuardStatus.md) |  | 
**customer_id_blocking_threshold** | **int** | Determines the unsuccessful payment threshold for Customer Id Blocking for profile | 
**card_testing_guard_expiry** | **int** | Determines Redis Expiry for Card Testing Guard for profile | 

## Example

```python
from hyperswitch.models.card_testing_guard_config import CardTestingGuardConfig

# TODO update the JSON string below
json = "{}"
# create an instance of CardTestingGuardConfig from a JSON string
card_testing_guard_config_instance = CardTestingGuardConfig.from_json(json)
# print the JSON string representation of the object
print(CardTestingGuardConfig.to_json())

# convert the object into a dict
card_testing_guard_config_dict = card_testing_guard_config_instance.to_dict()
# create an instance of CardTestingGuardConfig from a dict
card_testing_guard_config_from_dict = CardTestingGuardConfig.from_dict(card_testing_guard_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


