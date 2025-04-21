from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.surcharge_response_type_1_type import SurchargeResponseType1Type

if TYPE_CHECKING:
    from ..models.surcharge_percentage import SurchargePercentage


T = TypeVar("T", bound="SurchargeResponseType1")


@_attrs_define
class SurchargeResponseType1:
    """
    Attributes:
        type_ (SurchargeResponseType1Type):
        value (SurchargePercentage):
    """

    type_: SurchargeResponseType1Type
    value: "SurchargePercentage"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        value = self.value.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "value": value,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.surcharge_percentage import SurchargePercentage

        d = dict(src_dict)
        type_ = SurchargeResponseType1Type(d.pop("type"))

        value = SurchargePercentage.from_dict(d.pop("value"))

        surcharge_response_type_1 = cls(
            type_=type_,
            value=value,
        )

        surcharge_response_type_1.additional_properties = d
        return surcharge_response_type_1

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
