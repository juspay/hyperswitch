from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.comparison_type import ComparisonType

if TYPE_CHECKING:
    from ..models.comparison_metadata import ComparisonMetadata
    from ..models.value_type_type_0 import ValueTypeType0
    from ..models.value_type_type_1 import ValueTypeType1
    from ..models.value_type_type_2 import ValueTypeType2
    from ..models.value_type_type_3 import ValueTypeType3
    from ..models.value_type_type_4 import ValueTypeType4
    from ..models.value_type_type_5 import ValueTypeType5
    from ..models.value_type_type_6 import ValueTypeType6


T = TypeVar("T", bound="Comparison")


@_attrs_define
class Comparison:
    """Represents a single comparison condition.

    Attributes:
        lhs (str): The left hand side which will always be a domain input identifier like "payment.method.cardtype"
        comparison (ComparisonType): Conditional comparison type
        value (Union['ValueTypeType0', 'ValueTypeType1', 'ValueTypeType2', 'ValueTypeType3', 'ValueTypeType4',
            'ValueTypeType5', 'ValueTypeType6']): Represents a value in the DSL
        metadata (ComparisonMetadata): Additional metadata that the Static Analyzer and Backend does not touch.
            This can be used to store useful information for the frontend and is required for communication
            between the static analyzer and the frontend.
    """

    lhs: str
    comparison: ComparisonType
    value: Union[
        "ValueTypeType0",
        "ValueTypeType1",
        "ValueTypeType2",
        "ValueTypeType3",
        "ValueTypeType4",
        "ValueTypeType5",
        "ValueTypeType6",
    ]
    metadata: "ComparisonMetadata"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.value_type_type_0 import ValueTypeType0
        from ..models.value_type_type_1 import ValueTypeType1
        from ..models.value_type_type_2 import ValueTypeType2
        from ..models.value_type_type_3 import ValueTypeType3
        from ..models.value_type_type_4 import ValueTypeType4
        from ..models.value_type_type_5 import ValueTypeType5

        lhs = self.lhs

        comparison = self.comparison.value

        value: dict[str, Any]
        if isinstance(self.value, ValueTypeType0):
            value = self.value.to_dict()
        elif isinstance(self.value, ValueTypeType1):
            value = self.value.to_dict()
        elif isinstance(self.value, ValueTypeType2):
            value = self.value.to_dict()
        elif isinstance(self.value, ValueTypeType3):
            value = self.value.to_dict()
        elif isinstance(self.value, ValueTypeType4):
            value = self.value.to_dict()
        elif isinstance(self.value, ValueTypeType5):
            value = self.value.to_dict()
        else:
            value = self.value.to_dict()

        metadata = self.metadata.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "lhs": lhs,
                "comparison": comparison,
                "value": value,
                "metadata": metadata,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.comparison_metadata import ComparisonMetadata
        from ..models.value_type_type_0 import ValueTypeType0
        from ..models.value_type_type_1 import ValueTypeType1
        from ..models.value_type_type_2 import ValueTypeType2
        from ..models.value_type_type_3 import ValueTypeType3
        from ..models.value_type_type_4 import ValueTypeType4
        from ..models.value_type_type_5 import ValueTypeType5
        from ..models.value_type_type_6 import ValueTypeType6

        d = dict(src_dict)
        lhs = d.pop("lhs")

        comparison = ComparisonType(d.pop("comparison"))

        def _parse_value(
            data: object,
        ) -> Union[
            "ValueTypeType0",
            "ValueTypeType1",
            "ValueTypeType2",
            "ValueTypeType3",
            "ValueTypeType4",
            "ValueTypeType5",
            "ValueTypeType6",
        ]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_value_type_type_0 = ValueTypeType0.from_dict(data)

                return componentsschemas_value_type_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_value_type_type_1 = ValueTypeType1.from_dict(data)

                return componentsschemas_value_type_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_value_type_type_2 = ValueTypeType2.from_dict(data)

                return componentsschemas_value_type_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_value_type_type_3 = ValueTypeType3.from_dict(data)

                return componentsschemas_value_type_type_3
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_value_type_type_4 = ValueTypeType4.from_dict(data)

                return componentsschemas_value_type_type_4
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_value_type_type_5 = ValueTypeType5.from_dict(data)

                return componentsschemas_value_type_type_5
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_value_type_type_6 = ValueTypeType6.from_dict(data)

            return componentsschemas_value_type_type_6

        value = _parse_value(d.pop("value"))

        metadata = ComparisonMetadata.from_dict(d.pop("metadata"))

        comparison = cls(
            lhs=lhs,
            comparison=comparison,
            value=value,
            metadata=metadata,
        )

        comparison.additional_properties = d
        return comparison

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
