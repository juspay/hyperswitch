# IfStatement

Represents an IF statement with conditions and optional nested IF statements  ```text payment.method = card { payment.method.cardtype = (credit, debit) { payment.method.network = (amex, rupay, diners) } } ```

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**condition** | [**List[Comparison]**](Comparison.md) |  | 
**nested** | [**List[IfStatement]**](IfStatement.md) |  | [optional] 

## Example

```python
from hyperswitch.models.if_statement import IfStatement

# TODO update the JSON string below
json = "{}"
# create an instance of IfStatement from a JSON string
if_statement_instance = IfStatement.from_json(json)
# print the JSON string representation of the object
print(IfStatement.to_json())

# convert the object into a dict
if_statement_dict = if_statement_instance.to_dict()
# create an instance of IfStatement from a dict
if_statement_from_dict = IfStatement.from_dict(if_statement_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


