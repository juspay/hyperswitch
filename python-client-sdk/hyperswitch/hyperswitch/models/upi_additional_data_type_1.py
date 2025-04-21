from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.upi_intent_data import UpiIntentData


T = TypeVar("T", bound="UpiAdditionalDataType1")


@_attrs_define
class UpiAdditionalDataType1:
    """
    Attributes:
        upi_intent (UpiIntentData):
    """

    upi_intent: "UpiIntentData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        upi_intent = self.upi_intent.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "upi_intent": upi_intent,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.upi_intent_data import UpiIntentData

        d = dict(src_dict)
        upi_intent = UpiIntentData.from_dict(d.pop("upi_intent"))

        upi_additional_data_type_1 = cls(
            upi_intent=upi_intent,
        )

        upi_additional_data_type_1.additional_properties = d
        return upi_additional_data_type_1

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
