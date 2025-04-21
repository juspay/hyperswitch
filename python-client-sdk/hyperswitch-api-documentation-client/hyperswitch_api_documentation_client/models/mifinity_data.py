import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..types import UNSET, Unset

T = TypeVar("T", bound="MifinityData")


@_attrs_define
class MifinityData:
    """
    Attributes:
        date_of_birth (datetime.date):
        language_preference (Union[None, Unset, str]):
    """

    date_of_birth: datetime.date
    language_preference: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        date_of_birth = self.date_of_birth.isoformat()

        language_preference: Union[None, Unset, str]
        if isinstance(self.language_preference, Unset):
            language_preference = UNSET
        else:
            language_preference = self.language_preference

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "date_of_birth": date_of_birth,
            }
        )
        if language_preference is not UNSET:
            field_dict["language_preference"] = language_preference

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        date_of_birth = isoparse(d.pop("date_of_birth")).date()

        def _parse_language_preference(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        language_preference = _parse_language_preference(d.pop("language_preference", UNSET))

        mifinity_data = cls(
            date_of_birth=date_of_birth,
            language_preference=language_preference,
        )

        mifinity_data.additional_properties = d
        return mifinity_data

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
