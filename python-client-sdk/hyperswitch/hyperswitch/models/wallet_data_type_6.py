from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.go_pay_redirection import GoPayRedirection


T = TypeVar("T", bound="WalletDataType6")


@_attrs_define
class WalletDataType6:
    """
    Attributes:
        go_pay_redirect (GoPayRedirection):
    """

    go_pay_redirect: "GoPayRedirection"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        go_pay_redirect = self.go_pay_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "go_pay_redirect": go_pay_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.go_pay_redirection import GoPayRedirection

        d = dict(src_dict)
        go_pay_redirect = GoPayRedirection.from_dict(d.pop("go_pay_redirect"))

        wallet_data_type_6 = cls(
            go_pay_redirect=go_pay_redirect,
        )

        wallet_data_type_6.additional_properties = d
        return wallet_data_type_6

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
