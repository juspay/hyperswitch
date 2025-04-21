from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.ali_pay_redirection import AliPayRedirection


T = TypeVar("T", bound="WalletDataType1")


@_attrs_define
class WalletDataType1:
    """
    Attributes:
        ali_pay_redirect (AliPayRedirection):
    """

    ali_pay_redirect: "AliPayRedirection"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        ali_pay_redirect = self.ali_pay_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "ali_pay_redirect": ali_pay_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.ali_pay_redirection import AliPayRedirection

        d = dict(src_dict)
        ali_pay_redirect = AliPayRedirection.from_dict(d.pop("ali_pay_redirect"))

        wallet_data_type_1 = cls(
            ali_pay_redirect=ali_pay_redirect,
        )

        wallet_data_type_1.additional_properties = d
        return wallet_data_type_1

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
