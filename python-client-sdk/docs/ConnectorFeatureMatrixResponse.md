# ConnectorFeatureMatrixResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** | The name of the connector | 
**display_name** | **str** | The display name of the connector | [optional] 
**description** | **str** | The description of the connector | [optional] 
**category** | [**PaymentConnectorCategory**](PaymentConnectorCategory.md) |  | [optional] 
**supported_payment_methods** | [**List[SupportedPaymentMethod]**](SupportedPaymentMethod.md) | The list of payment methods supported by the connector | 
**supported_webhook_flows** | [**List[EventClass]**](EventClass.md) | The list of webhook flows supported by the connector | [optional] 

## Example

```python
from hyperswitch.models.connector_feature_matrix_response import ConnectorFeatureMatrixResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorFeatureMatrixResponse from a JSON string
connector_feature_matrix_response_instance = ConnectorFeatureMatrixResponse.from_json(json)
# print the JSON string representation of the object
print(ConnectorFeatureMatrixResponse.to_json())

# convert the object into a dict
connector_feature_matrix_response_dict = connector_feature_matrix_response_instance.to_dict()
# create an instance of ConnectorFeatureMatrixResponse from a dict
connector_feature_matrix_response_from_dict = ConnectorFeatureMatrixResponse.from_dict(connector_feature_matrix_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


