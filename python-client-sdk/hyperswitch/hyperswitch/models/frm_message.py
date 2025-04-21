from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="FrmMessage")


@_attrs_define
class FrmMessage:
    """frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its
    None

        Attributes:
            frm_name (str):
            frm_transaction_id (Union[None, Unset, str]):
            frm_transaction_type (Union[None, Unset, str]):
            frm_status (Union[None, Unset, str]):
            frm_score (Union[None, Unset, int]):
            frm_reason (Union[Unset, Any]):
            frm_error (Union[None, Unset, str]):
    """

    frm_name: str
    frm_transaction_id: Union[None, Unset, str] = UNSET
    frm_transaction_type: Union[None, Unset, str] = UNSET
    frm_status: Union[None, Unset, str] = UNSET
    frm_score: Union[None, Unset, int] = UNSET
    frm_reason: Union[Unset, Any] = UNSET
    frm_error: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        frm_name = self.frm_name

        frm_transaction_id: Union[None, Unset, str]
        if isinstance(self.frm_transaction_id, Unset):
            frm_transaction_id = UNSET
        else:
            frm_transaction_id = self.frm_transaction_id

        frm_transaction_type: Union[None, Unset, str]
        if isinstance(self.frm_transaction_type, Unset):
            frm_transaction_type = UNSET
        else:
            frm_transaction_type = self.frm_transaction_type

        frm_status: Union[None, Unset, str]
        if isinstance(self.frm_status, Unset):
            frm_status = UNSET
        else:
            frm_status = self.frm_status

        frm_score: Union[None, Unset, int]
        if isinstance(self.frm_score, Unset):
            frm_score = UNSET
        else:
            frm_score = self.frm_score

        frm_reason = self.frm_reason

        frm_error: Union[None, Unset, str]
        if isinstance(self.frm_error, Unset):
            frm_error = UNSET
        else:
            frm_error = self.frm_error

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "frm_name": frm_name,
            }
        )
        if frm_transaction_id is not UNSET:
            field_dict["frm_transaction_id"] = frm_transaction_id
        if frm_transaction_type is not UNSET:
            field_dict["frm_transaction_type"] = frm_transaction_type
        if frm_status is not UNSET:
            field_dict["frm_status"] = frm_status
        if frm_score is not UNSET:
            field_dict["frm_score"] = frm_score
        if frm_reason is not UNSET:
            field_dict["frm_reason"] = frm_reason
        if frm_error is not UNSET:
            field_dict["frm_error"] = frm_error

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        frm_name = d.pop("frm_name")

        def _parse_frm_transaction_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        frm_transaction_id = _parse_frm_transaction_id(d.pop("frm_transaction_id", UNSET))

        def _parse_frm_transaction_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        frm_transaction_type = _parse_frm_transaction_type(d.pop("frm_transaction_type", UNSET))

        def _parse_frm_status(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        frm_status = _parse_frm_status(d.pop("frm_status", UNSET))

        def _parse_frm_score(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        frm_score = _parse_frm_score(d.pop("frm_score", UNSET))

        frm_reason = d.pop("frm_reason", UNSET)

        def _parse_frm_error(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        frm_error = _parse_frm_error(d.pop("frm_error", UNSET))

        frm_message = cls(
            frm_name=frm_name,
            frm_transaction_id=frm_transaction_id,
            frm_transaction_type=frm_transaction_type,
            frm_status=frm_status,
            frm_score=frm_score,
            frm_reason=frm_reason,
            frm_error=frm_error,
        )

        frm_message.additional_properties = d
        return frm_message

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
