from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.google_pay_redirect_data import GooglePayRedirectData


T = TypeVar("T", bound="WalletDataType13")


@_attrs_define
class WalletDataType13:
    """
    Attributes:
        google_pay_redirect (GooglePayRedirectData):
    """

    google_pay_redirect: "GooglePayRedirectData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        google_pay_redirect = self.google_pay_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "google_pay_redirect": google_pay_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.google_pay_redirect_data import GooglePayRedirectData

        d = dict(src_dict)
        google_pay_redirect = GooglePayRedirectData.from_dict(d.pop("google_pay_redirect"))

        wallet_data_type_13 = cls(
            google_pay_redirect=google_pay_redirect,
        )

        wallet_data_type_13.additional_properties = d
        return wallet_data_type_13

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
