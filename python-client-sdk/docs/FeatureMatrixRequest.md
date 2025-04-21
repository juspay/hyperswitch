# FeatureMatrixRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connectors** | [**List[Connector]**](Connector.md) |  | [optional] 

## Example

```python
from hyperswitch.models.feature_matrix_request import FeatureMatrixRequest

# TODO update the JSON string below
json = "{}"
# create an instance of FeatureMatrixRequest from a JSON string
feature_matrix_request_instance = FeatureMatrixRequest.from_json(json)
# print the JSON string representation of the object
print(FeatureMatrixRequest.to_json())

# convert the object into a dict
feature_matrix_request_dict = feature_matrix_request_instance.to_dict()
# create an instance of FeatureMatrixRequest from a dict
feature_matrix_request_from_dict = FeatureMatrixRequest.from_dict(feature_matrix_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


