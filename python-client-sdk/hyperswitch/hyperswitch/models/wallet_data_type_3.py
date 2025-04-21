from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.amazon_pay_redirect_data import AmazonPayRedirectData


T = TypeVar("T", bound="WalletDataType3")


@_attrs_define
class WalletDataType3:
    """
    Attributes:
        amazon_pay_redirect (AmazonPayRedirectData):
    """

    amazon_pay_redirect: "AmazonPayRedirectData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        amazon_pay_redirect = self.amazon_pay_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "amazon_pay_redirect": amazon_pay_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.amazon_pay_redirect_data import AmazonPayRedirectData

        d = dict(src_dict)
        amazon_pay_redirect = AmazonPayRedirectData.from_dict(d.pop("amazon_pay_redirect"))

        wallet_data_type_3 = cls(
            amazon_pay_redirect=amazon_pay_redirect,
        )

        wallet_data_type_3.additional_properties = d
        return wallet_data_type_3

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
