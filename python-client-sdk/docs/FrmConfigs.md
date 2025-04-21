# FrmConfigs

Details of FrmConfigs are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**gateway** | [**ConnectorType**](ConnectorType.md) |  | 
**payment_methods** | [**List[FrmPaymentMethod]**](FrmPaymentMethod.md) | payment methods that can be used in the payment | 

## Example

```python
from hyperswitch.models.frm_configs import FrmConfigs

# TODO update the JSON string below
json = "{}"
# create an instance of FrmConfigs from a JSON string
frm_configs_instance = FrmConfigs.from_json(json)
# print the JSON string representation of the object
print(FrmConfigs.to_json())

# convert the object into a dict
frm_configs_dict = frm_configs_instance.to_dict()
# create an instance of FrmConfigs from a dict
frm_configs_from_dict = FrmConfigs.from_dict(frm_configs_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


