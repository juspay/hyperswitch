from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.ali_pay_hk_redirection import AliPayHkRedirection


T = TypeVar("T", bound="WalletDataType2")


@_attrs_define
class WalletDataType2:
    """
    Attributes:
        ali_pay_hk_redirect (AliPayHkRedirection):
    """

    ali_pay_hk_redirect: "AliPayHkRedirection"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        ali_pay_hk_redirect = self.ali_pay_hk_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "ali_pay_hk_redirect": ali_pay_hk_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.ali_pay_hk_redirection import AliPayHkRedirection

        d = dict(src_dict)
        ali_pay_hk_redirect = AliPayHkRedirection.from_dict(d.pop("ali_pay_hk_redirect"))

        wallet_data_type_2 = cls(
            ali_pay_hk_redirect=ali_pay_hk_redirect,
        )

        wallet_data_type_2.additional_properties = d
        return wallet_data_type_2

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
