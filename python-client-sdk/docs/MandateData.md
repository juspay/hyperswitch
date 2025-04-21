# MandateData

Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**update_mandate_id** | **str** | A way to update the mandate&#39;s payment method details | [optional] 
**customer_acceptance** | [**CustomerAcceptance**](CustomerAcceptance.md) |  | [optional] 
**mandate_type** | [**MandateType**](MandateType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.mandate_data import MandateData

# TODO update the JSON string below
json = "{}"
# create an instance of MandateData from a JSON string
mandate_data_instance = MandateData.from_json(json)
# print the JSON string representation of the object
print(MandateData.to_json())

# convert the object into a dict
mandate_data_dict = mandate_data_instance.to_dict()
# create an instance of MandateData from a dict
mandate_data_from_dict = MandateData.from_dict(mandate_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


