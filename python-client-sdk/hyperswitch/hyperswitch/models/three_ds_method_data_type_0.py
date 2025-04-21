from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.three_ds_method_data_type_0_three_ds_method_key import ThreeDsMethodDataType0ThreeDsMethodKey
from ..types import UNSET, Unset

T = TypeVar("T", bound="ThreeDsMethodDataType0")


@_attrs_define
class ThreeDsMethodDataType0:
    """
    Attributes:
        three_ds_method_data_submission (bool): Whether ThreeDS method data submission is required
        three_ds_method_key (ThreeDsMethodDataType0ThreeDsMethodKey):
        three_ds_method_data (Union[None, Unset, str]): ThreeDS method data
        three_ds_method_url (Union[None, Unset, str]): ThreeDS method url
    """

    three_ds_method_data_submission: bool
    three_ds_method_key: ThreeDsMethodDataType0ThreeDsMethodKey
    three_ds_method_data: Union[None, Unset, str] = UNSET
    three_ds_method_url: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        three_ds_method_data_submission = self.three_ds_method_data_submission

        three_ds_method_key = self.three_ds_method_key.value

        three_ds_method_data: Union[None, Unset, str]
        if isinstance(self.three_ds_method_data, Unset):
            three_ds_method_data = UNSET
        else:
            three_ds_method_data = self.three_ds_method_data

        three_ds_method_url: Union[None, Unset, str]
        if isinstance(self.three_ds_method_url, Unset):
            three_ds_method_url = UNSET
        else:
            three_ds_method_url = self.three_ds_method_url

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "three_ds_method_data_submission": three_ds_method_data_submission,
                "three_ds_method_key": three_ds_method_key,
            }
        )
        if three_ds_method_data is not UNSET:
            field_dict["three_ds_method_data"] = three_ds_method_data
        if three_ds_method_url is not UNSET:
            field_dict["three_ds_method_url"] = three_ds_method_url

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        three_ds_method_data_submission = d.pop("three_ds_method_data_submission")

        three_ds_method_key = ThreeDsMethodDataType0ThreeDsMethodKey(d.pop("three_ds_method_key"))

        def _parse_three_ds_method_data(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        three_ds_method_data = _parse_three_ds_method_data(d.pop("three_ds_method_data", UNSET))

        def _parse_three_ds_method_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        three_ds_method_url = _parse_three_ds_method_url(d.pop("three_ds_method_url", UNSET))

        three_ds_method_data_type_0 = cls(
            three_ds_method_data_submission=three_ds_method_data_submission,
            three_ds_method_key=three_ds_method_key,
            three_ds_method_data=three_ds_method_data,
            three_ds_method_url=three_ds_method_url,
        )

        three_ds_method_data_type_0.additional_properties = d
        return three_ds_method_data_type_0

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
