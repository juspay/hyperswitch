from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.comparison_type import ComparisonType

T = TypeVar("T", bound="NumberComparison")


@_attrs_define
class NumberComparison:
    """Represents a number comparison for "NumberComparisonArrayValue"

    Attributes:
        comparison_type (ComparisonType): Conditional comparison type
        number (int): This Unit struct represents MinorUnit in which core amount works
    """

    comparison_type: ComparisonType
    number: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        comparison_type = self.comparison_type.value

        number = self.number

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "comparisonType": comparison_type,
                "number": number,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        comparison_type = ComparisonType(d.pop("comparisonType"))

        number = d.pop("number")

        number_comparison = cls(
            comparison_type=comparison_type,
            number=number,
        )

        number_comparison.additional_properties = d
        return number_comparison

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
