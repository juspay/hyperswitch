# FeatureMatrixListResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_count** | **int** | The number of connectors included in the response | 
**connectors** | [**List[ConnectorFeatureMatrixResponse]**](ConnectorFeatureMatrixResponse.md) |  | 

## Example

```python
from hyperswitch.models.feature_matrix_list_response import FeatureMatrixListResponse

# TODO update the JSON string below
json = "{}"
# create an instance of FeatureMatrixListResponse from a JSON string
feature_matrix_list_response_instance = FeatureMatrixListResponse.from_json(json)
# print the JSON string representation of the object
print(FeatureMatrixListResponse.to_json())

# convert the object into a dict
feature_matrix_list_response_dict = feature_matrix_list_response_instance.to_dict()
# create an instance of FeatureMatrixListResponse from a dict
feature_matrix_list_response_from_dict = FeatureMatrixListResponse.from_dict(feature_matrix_list_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


