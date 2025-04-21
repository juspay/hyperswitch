from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="MerchantRecipientDataType0")


@_attrs_define
class MerchantRecipientDataType0:
    """
    Attributes:
        connector_recipient_id (Union[None, str]):
    """

    connector_recipient_id: Union[None, str]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector_recipient_id: Union[None, str]
        connector_recipient_id = self.connector_recipient_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector_recipient_id": connector_recipient_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_connector_recipient_id(data: object) -> Union[None, str]:
            if data is None:
                return data
            return cast(Union[None, str], data)

        connector_recipient_id = _parse_connector_recipient_id(d.pop("connector_recipient_id"))

        merchant_recipient_data_type_0 = cls(
            connector_recipient_id=connector_recipient_id,
        )

        merchant_recipient_data_type_0.additional_properties = d
        return merchant_recipient_data_type_0

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
