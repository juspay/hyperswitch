from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.connector_selection_type_0 import ConnectorSelectionType0
    from ..models.connector_selection_type_1 import ConnectorSelectionType1
    from ..models.if_statement import IfStatement


T = TypeVar("T", bound="RuleConnectorSelection")


@_attrs_define
class RuleConnectorSelection:
    """Represents a rule

    ```text
    rule_name: [stripe, adyen, checkout]
    {
    payment.method = card {
    payment.method.cardtype = (credit, debit) {
    payment.method.network = (amex, rupay, diners)
    }

    payment.method.cardtype = credit
    }
    }
    ```

        Attributes:
            name (str):
            connector_selection (Union['ConnectorSelectionType0', 'ConnectorSelectionType1']):
            statements (list['IfStatement']):
    """

    name: str
    connector_selection: Union["ConnectorSelectionType0", "ConnectorSelectionType1"]
    statements: list["IfStatement"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.connector_selection_type_0 import ConnectorSelectionType0

        name = self.name

        connector_selection: dict[str, Any]
        if isinstance(self.connector_selection, ConnectorSelectionType0):
            connector_selection = self.connector_selection.to_dict()
        else:
            connector_selection = self.connector_selection.to_dict()

        statements = []
        for statements_item_data in self.statements:
            statements_item = statements_item_data.to_dict()
            statements.append(statements_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "connectorSelection": connector_selection,
                "statements": statements,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.connector_selection_type_0 import ConnectorSelectionType0
        from ..models.connector_selection_type_1 import ConnectorSelectionType1
        from ..models.if_statement import IfStatement

        d = dict(src_dict)
        name = d.pop("name")

        def _parse_connector_selection(data: object) -> Union["ConnectorSelectionType0", "ConnectorSelectionType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_connector_selection_type_0 = ConnectorSelectionType0.from_dict(data)

                return componentsschemas_connector_selection_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_connector_selection_type_1 = ConnectorSelectionType1.from_dict(data)

            return componentsschemas_connector_selection_type_1

        connector_selection = _parse_connector_selection(d.pop("connectorSelection"))

        statements = []
        _statements = d.pop("statements")
        for statements_item_data in _statements:
            statements_item = IfStatement.from_dict(statements_item_data)

            statements.append(statements_item)

        rule_connector_selection = cls(
            name=name,
            connector_selection=connector_selection,
            statements=statements,
        )

        rule_connector_selection.additional_properties = d
        return rule_connector_selection

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
