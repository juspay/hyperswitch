from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="LabelInformation")


@_attrs_define
class LabelInformation:
    """
    Attributes:
        label (str):
        target_count (int):
        target_time (int):
        mca_id (str):
    """

    label: str
    target_count: int
    target_time: int
    mca_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        label = self.label

        target_count = self.target_count

        target_time = self.target_time

        mca_id = self.mca_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "label": label,
                "target_count": target_count,
                "target_time": target_time,
                "mca_id": mca_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        label = d.pop("label")

        target_count = d.pop("target_count")

        target_time = d.pop("target_time")

        mca_id = d.pop("mca_id")

        label_information = cls(
            label=label,
            target_count=target_count,
            target_time=target_time,
            mca_id=mca_id,
        )

        label_information.additional_properties = d
        return label_information

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
