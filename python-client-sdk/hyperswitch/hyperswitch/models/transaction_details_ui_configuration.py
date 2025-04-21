from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="TransactionDetailsUiConfiguration")


@_attrs_define
class TransactionDetailsUiConfiguration:
    """
    Attributes:
        position (Union[None, Unset, int]): Position of the key-value pair in the UI Example: 5.
        is_key_bold (Union[None, Unset, bool]): Whether the key should be bold Default: False. Example: True.
        is_value_bold (Union[None, Unset, bool]): Whether the value should be bold Default: False. Example: True.
    """

    position: Union[None, Unset, int] = UNSET
    is_key_bold: Union[None, Unset, bool] = False
    is_value_bold: Union[None, Unset, bool] = False
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        position: Union[None, Unset, int]
        if isinstance(self.position, Unset):
            position = UNSET
        else:
            position = self.position

        is_key_bold: Union[None, Unset, bool]
        if isinstance(self.is_key_bold, Unset):
            is_key_bold = UNSET
        else:
            is_key_bold = self.is_key_bold

        is_value_bold: Union[None, Unset, bool]
        if isinstance(self.is_value_bold, Unset):
            is_value_bold = UNSET
        else:
            is_value_bold = self.is_value_bold

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if position is not UNSET:
            field_dict["position"] = position
        if is_key_bold is not UNSET:
            field_dict["is_key_bold"] = is_key_bold
        if is_value_bold is not UNSET:
            field_dict["is_value_bold"] = is_value_bold

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_position(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        position = _parse_position(d.pop("position", UNSET))

        def _parse_is_key_bold(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_key_bold = _parse_is_key_bold(d.pop("is_key_bold", UNSET))

        def _parse_is_value_bold(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_value_bold = _parse_is_value_bold(d.pop("is_value_bold", UNSET))

        transaction_details_ui_configuration = cls(
            position=position,
            is_key_bold=is_key_bold,
            is_value_bold=is_value_bold,
        )

        transaction_details_ui_configuration.additional_properties = d
        return transaction_details_ui_configuration

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
