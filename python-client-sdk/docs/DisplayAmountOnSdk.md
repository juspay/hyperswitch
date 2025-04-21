# DisplayAmountOnSdk


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**net_amount** | **str** | net amount &#x3D; amount + order_tax_amount + shipping_cost | 
**order_tax_amount** | **str** | order tax amount calculated by tax connectors | 
**shipping_cost** | **str** | shipping cost for the order | 

## Example

```python
from hyperswitch.models.display_amount_on_sdk import DisplayAmountOnSdk

# TODO update the JSON string below
json = "{}"
# create an instance of DisplayAmountOnSdk from a JSON string
display_amount_on_sdk_instance = DisplayAmountOnSdk.from_json(json)
# print the JSON string representation of the object
print(DisplayAmountOnSdk.to_json())

# convert the object into a dict
display_amount_on_sdk_dict = display_amount_on_sdk_instance.to_dict()
# create an instance of DisplayAmountOnSdk from a dict
display_amount_on_sdk_from_dict = DisplayAmountOnSdk.from_dict(display_amount_on_sdk_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


