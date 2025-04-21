from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ExtendedCardInfoConfig")


@_attrs_define
class ExtendedCardInfoConfig:
    """
    Attributes:
        public_key (str): Merchant public key
        ttl_in_secs (Union[Unset, int]): TTL for extended card info Default: 900.
    """

    public_key: str
    ttl_in_secs: Union[Unset, int] = 900
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        public_key = self.public_key

        ttl_in_secs = self.ttl_in_secs

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "public_key": public_key,
            }
        )
        if ttl_in_secs is not UNSET:
            field_dict["ttl_in_secs"] = ttl_in_secs

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        public_key = d.pop("public_key")

        ttl_in_secs = d.pop("ttl_in_secs", UNSET)

        extended_card_info_config = cls(
            public_key=public_key,
            ttl_in_secs=ttl_in_secs,
        )

        extended_card_info_config.additional_properties = d
        return extended_card_info_config

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
