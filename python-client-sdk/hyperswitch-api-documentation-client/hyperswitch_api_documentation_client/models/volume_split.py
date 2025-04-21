from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.volume_split_type import VolumeSplitType

if TYPE_CHECKING:
    from ..models.connector_volume_split import ConnectorVolumeSplit


T = TypeVar("T", bound="VolumeSplit")


@_attrs_define
class VolumeSplit:
    """
    Attributes:
        type_ (VolumeSplitType):
        data (list['ConnectorVolumeSplit']):
    """

    type_: VolumeSplitType
    data: list["ConnectorVolumeSplit"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        data = []
        for data_item_data in self.data:
            data_item = data_item_data.to_dict()
            data.append(data_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "data": data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.connector_volume_split import ConnectorVolumeSplit

        d = dict(src_dict)
        type_ = VolumeSplitType(d.pop("type"))

        data = []
        _data = d.pop("data")
        for data_item_data in _data:
            data_item = ConnectorVolumeSplit.from_dict(data_item_data)

            data.append(data_item)

        volume_split = cls(
            type_=type_,
            data=data,
        )

        volume_split.additional_properties = d
        return volume_split

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
