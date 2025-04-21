# AdyenSplitData

Fee information for Split Payments to be charged on the payment being collected for Adyen

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**store** | **str** | The store identifier | [optional] 
**split_items** | [**List[AdyenSplitItem]**](AdyenSplitItem.md) | Data for the split items | 

## Example

```python
from hyperswitch.models.adyen_split_data import AdyenSplitData

# TODO update the JSON string below
json = "{}"
# create an instance of AdyenSplitData from a JSON string
adyen_split_data_instance = AdyenSplitData.from_json(json)
# print the JSON string representation of the object
print(AdyenSplitData.to_json())

# convert the object into a dict
adyen_split_data_dict = adyen_split_data_instance.to_dict()
# create an instance of AdyenSplitData from a dict
adyen_split_data_from_dict = AdyenSplitData.from_dict(adyen_split_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


