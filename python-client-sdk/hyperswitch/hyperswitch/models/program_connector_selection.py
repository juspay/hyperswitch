from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.connector_selection_type_0 import ConnectorSelectionType0
    from ..models.connector_selection_type_1 import ConnectorSelectionType1
    from ..models.program_connector_selection_metadata import ProgramConnectorSelectionMetadata
    from ..models.rule_connector_selection import RuleConnectorSelection


T = TypeVar("T", bound="ProgramConnectorSelection")


@_attrs_define
class ProgramConnectorSelection:
    """The program, having a default connector selection and
    a bunch of rules. Also can hold arbitrary metadata.

        Attributes:
            default_selection (Union['ConnectorSelectionType0', 'ConnectorSelectionType1']):
            rules (RuleConnectorSelection): Represents a rule

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
            metadata (ProgramConnectorSelectionMetadata):
    """

    default_selection: Union["ConnectorSelectionType0", "ConnectorSelectionType1"]
    rules: "RuleConnectorSelection"
    metadata: "ProgramConnectorSelectionMetadata"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.connector_selection_type_0 import ConnectorSelectionType0

        default_selection: dict[str, Any]
        if isinstance(self.default_selection, ConnectorSelectionType0):
            default_selection = self.default_selection.to_dict()
        else:
            default_selection = self.default_selection.to_dict()

        rules = self.rules.to_dict()

        metadata = self.metadata.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "defaultSelection": default_selection,
                "rules": rules,
                "metadata": metadata,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.connector_selection_type_0 import ConnectorSelectionType0
        from ..models.connector_selection_type_1 import ConnectorSelectionType1
        from ..models.program_connector_selection_metadata import ProgramConnectorSelectionMetadata
        from ..models.rule_connector_selection import RuleConnectorSelection

        d = dict(src_dict)

        def _parse_default_selection(data: object) -> Union["ConnectorSelectionType0", "ConnectorSelectionType1"]:
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

        default_selection = _parse_default_selection(d.pop("defaultSelection"))

        rules = RuleConnectorSelection.from_dict(d.pop("rules"))

        metadata = ProgramConnectorSelectionMetadata.from_dict(d.pop("metadata"))

        program_connector_selection = cls(
            default_selection=default_selection,
            rules=rules,
            metadata=metadata,
        )

        program_connector_selection.additional_properties = d
        return program_connector_selection

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
