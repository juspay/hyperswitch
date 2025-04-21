from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="CurrentBlockThreshold")


@_attrs_define
class CurrentBlockThreshold:
    """
    Attributes:
        duration_in_mins (Union[None, Unset, int]):
        max_total_count (Union[None, Unset, int]):
    """

    duration_in_mins: Union[None, Unset, int] = UNSET
    max_total_count: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        duration_in_mins: Union[None, Unset, int]
        if isinstance(self.duration_in_mins, Unset):
            duration_in_mins = UNSET
        else:
            duration_in_mins = self.duration_in_mins

        max_total_count: Union[None, Unset, int]
        if isinstance(self.max_total_count, Unset):
            max_total_count = UNSET
        else:
            max_total_count = self.max_total_count

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if duration_in_mins is not UNSET:
            field_dict["duration_in_mins"] = duration_in_mins
        if max_total_count is not UNSET:
            field_dict["max_total_count"] = max_total_count

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_duration_in_mins(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        duration_in_mins = _parse_duration_in_mins(d.pop("duration_in_mins", UNSET))

        def _parse_max_total_count(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        max_total_count = _parse_max_total_count(d.pop("max_total_count", UNSET))

        current_block_threshold = cls(
            duration_in_mins=duration_in_mins,
            max_total_count=max_total_count,
        )

        current_block_threshold.additional_properties = d
        return current_block_threshold

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
