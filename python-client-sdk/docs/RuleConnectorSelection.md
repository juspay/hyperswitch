# RuleConnectorSelection

Represents a rule  ```text rule_name: [stripe, adyen, checkout] { payment.method = card { payment.method.cardtype = (credit, debit) { payment.method.network = (amex, rupay, diners) }  payment.method.cardtype = credit } } ```

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** |  | 
**connector_selection** | [**ConnectorSelection**](ConnectorSelection.md) |  | 
**statements** | [**List[IfStatement]**](IfStatement.md) |  | 

## Example

```python
from hyperswitch.models.rule_connector_selection import RuleConnectorSelection

# TODO update the JSON string below
json = "{}"
# create an instance of RuleConnectorSelection from a JSON string
rule_connector_selection_instance = RuleConnectorSelection.from_json(json)
# print the JSON string representation of the object
print(RuleConnectorSelection.to_json())

# convert the object into a dict
rule_connector_selection_dict = rule_connector_selection_instance.to_dict()
# create an instance of RuleConnectorSelection from a dict
rule_connector_selection_from_dict = RuleConnectorSelection.from_dict(rule_connector_selection_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


