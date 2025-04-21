# OrderDetailsWithAmount


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**product_name** | **str** | Name of the product that is being purchased | 
**quantity** | **int** | The quantity of the product to be purchased | 
**amount** | **int** | the amount per quantity of product | 
**tax_rate** | **float** | tax rate applicable to the product | [optional] 
**total_tax_amount** | **int** | total tax amount applicable to the product | [optional] 
**requires_shipping** | **bool** |  | [optional] 
**product_img_link** | **str** | The image URL of the product | [optional] 
**product_id** | **str** | ID of the product that is being purchased | [optional] 
**category** | **str** | Category of the product that is being purchased | [optional] 
**sub_category** | **str** | Sub category of the product that is being purchased | [optional] 
**brand** | **str** | Brand of the product that is being purchased | [optional] 
**product_type** | [**ProductType**](ProductType.md) |  | [optional] 
**product_tax_code** | **str** | The tax code for the product | [optional] 

## Example

```python
from hyperswitch.models.order_details_with_amount import OrderDetailsWithAmount

# TODO update the JSON string below
json = "{}"
# create an instance of OrderDetailsWithAmount from a JSON string
order_details_with_amount_instance = OrderDetailsWithAmount.from_json(json)
# print the JSON string representation of the object
print(OrderDetailsWithAmount.to_json())

# convert the object into a dict
order_details_with_amount_dict = order_details_with_amount_instance.to_dict()
# create an instance of OrderDetailsWithAmount from a dict
order_details_with_amount_from_dict = OrderDetailsWithAmount.from_dict(order_details_with_amount_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


