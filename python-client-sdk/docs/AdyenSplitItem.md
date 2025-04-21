# AdyenSplitItem

Data for the split items

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount** | **int** | The amount of the split item | 
**split_type** | [**AdyenSplitType**](AdyenSplitType.md) |  | 
**account** | **str** | The unique identifier of the account to which the split amount is allocated. | [optional] 
**reference** | **str** | Unique Identifier for the split item | 
**description** | **str** | Description for the part of the payment that will be allocated to the specified account. | [optional] 

## Example

```python
from hyperswitch.models.adyen_split_item import AdyenSplitItem

# TODO update the JSON string below
json = "{}"
# create an instance of AdyenSplitItem from a JSON string
adyen_split_item_instance = AdyenSplitItem.from_json(json)
# print the JSON string representation of the object
print(AdyenSplitItem.to_json())

# convert the object into a dict
adyen_split_item_dict = adyen_split_item_instance.to_dict()
# create an instance of AdyenSplitItem from a dict
adyen_split_item_from_dict = AdyenSplitItem.from_dict(adyen_split_item_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


