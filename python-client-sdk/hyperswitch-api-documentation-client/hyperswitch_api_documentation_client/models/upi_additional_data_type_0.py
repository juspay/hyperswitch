from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.upi_collect_additional_data import UpiCollectAdditionalData


T = TypeVar("T", bound="UpiAdditionalDataType0")


@_attrs_define
class UpiAdditionalDataType0:
    """
    Attributes:
        upi_collect (UpiCollectAdditionalData):
    """

    upi_collect: "UpiCollectAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        upi_collect = self.upi_collect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "upi_collect": upi_collect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.upi_collect_additional_data import UpiCollectAdditionalData

        d = dict(src_dict)
        upi_collect = UpiCollectAdditionalData.from_dict(d.pop("upi_collect"))

        upi_additional_data_type_0 = cls(
            upi_collect=upi_collect,
        )

        upi_additional_data_type_0.additional_properties = d
        return upi_additional_data_type_0

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
