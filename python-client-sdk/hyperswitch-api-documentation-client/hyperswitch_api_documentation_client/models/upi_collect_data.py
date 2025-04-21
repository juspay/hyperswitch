from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UpiCollectData")


@_attrs_define
class UpiCollectData:
    """
    Attributes:
        vpa_id (Union[None, Unset, str]):  Example: successtest@iata.
    """

    vpa_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        vpa_id: Union[None, Unset, str]
        if isinstance(self.vpa_id, Unset):
            vpa_id = UNSET
        else:
            vpa_id = self.vpa_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if vpa_id is not UNSET:
            field_dict["vpa_id"] = vpa_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_vpa_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        vpa_id = _parse_vpa_id(d.pop("vpa_id", UNSET))

        upi_collect_data = cls(
            vpa_id=vpa_id,
        )

        upi_collect_data.additional_properties = d
        return upi_collect_data

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
