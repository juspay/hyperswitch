# CardNetworkTypes


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_network** | [**CardNetwork**](CardNetwork.md) |  | [optional] 
**surcharge_details** | [**SurchargeDetailsResponse**](SurchargeDetailsResponse.md) |  | [optional] 
**eligible_connectors** | **List[str]** | The list of eligible connectors for a given card network | 

## Example

```python
from hyperswitch.models.card_network_types import CardNetworkTypes

# TODO update the JSON string below
json = "{}"
# create an instance of CardNetworkTypes from a JSON string
card_network_types_instance = CardNetworkTypes.from_json(json)
# print the JSON string representation of the object
print(CardNetworkTypes.to_json())

# convert the object into a dict
card_network_types_dict = card_network_types_instance.to_dict()
# create an instance of CardNetworkTypes from a dict
card_network_types_from_dict = CardNetworkTypes.from_dict(card_network_types_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


