# GpayTokenizationSpecification


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** | The token specification type(ex: PAYMENT_GATEWAY) | 
**parameters** | [**GpayTokenParameters**](GpayTokenParameters.md) |  | 

## Example

```python
from hyperswitch.models.gpay_tokenization_specification import GpayTokenizationSpecification

# TODO update the JSON string below
json = "{}"
# create an instance of GpayTokenizationSpecification from a JSON string
gpay_tokenization_specification_instance = GpayTokenizationSpecification.from_json(json)
# print the JSON string representation of the object
print(GpayTokenizationSpecification.to_json())

# convert the object into a dict
gpay_tokenization_specification_dict = gpay_tokenization_specification_instance.to_dict()
# create an instance of GpayTokenizationSpecification from a dict
gpay_tokenization_specification_from_dict = GpayTokenizationSpecification.from_dict(gpay_tokenization_specification_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


