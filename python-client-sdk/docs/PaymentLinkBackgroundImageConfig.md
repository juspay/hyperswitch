# PaymentLinkBackgroundImageConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**url** | **str** | URL of the image | 
**position** | [**ElementPosition**](ElementPosition.md) |  | [optional] 
**size** | [**ElementSize**](ElementSize.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payment_link_background_image_config import PaymentLinkBackgroundImageConfig

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentLinkBackgroundImageConfig from a JSON string
payment_link_background_image_config_instance = PaymentLinkBackgroundImageConfig.from_json(json)
# print the JSON string representation of the object
print(PaymentLinkBackgroundImageConfig.to_json())

# convert the object into a dict
payment_link_background_image_config_dict = payment_link_background_image_config_instance.to_dict()
# create an instance of PaymentLinkBackgroundImageConfig from a dict
payment_link_background_image_config_from_dict = PaymentLinkBackgroundImageConfig.from_dict(payment_link_background_image_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


