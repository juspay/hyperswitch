from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="MerchantAccountDataType1Bacs")


@_attrs_define
class MerchantAccountDataType1Bacs:
    """
    Attributes:
        account_number (str):
        sort_code (str):
        name (str):
        connector_recipient_id (Union[None, Unset, str]):
    """

    account_number: str
    sort_code: str
    name: str
    connector_recipient_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        account_number = self.account_number

        sort_code = self.sort_code

        name = self.name

        connector_recipient_id: Union[None, Unset, str]
        if isinstance(self.connector_recipient_id, Unset):
            connector_recipient_id = UNSET
        else:
            connector_recipient_id = self.connector_recipient_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_number": account_number,
                "sort_code": sort_code,
                "name": name,
            }
        )
        if connector_recipient_id is not UNSET:
            field_dict["connector_recipient_id"] = connector_recipient_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        account_number = d.pop("account_number")

        sort_code = d.pop("sort_code")

        name = d.pop("name")

        def _parse_connector_recipient_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_recipient_id = _parse_connector_recipient_id(d.pop("connector_recipient_id", UNSET))

        merchant_account_data_type_1_bacs = cls(
            account_number=account_number,
            sort_code=sort_code,
            name=name,
            connector_recipient_id=connector_recipient_id,
        )

        merchant_account_data_type_1_bacs.additional_properties = d
        return merchant_account_data_type_1_bacs

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
