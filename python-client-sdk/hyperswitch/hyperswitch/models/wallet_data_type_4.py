from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.momo_redirection import MomoRedirection


T = TypeVar("T", bound="WalletDataType4")


@_attrs_define
class WalletDataType4:
    """
    Attributes:
        momo_redirect (MomoRedirection):
    """

    momo_redirect: "MomoRedirection"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        momo_redirect = self.momo_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "momo_redirect": momo_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.momo_redirection import MomoRedirection

        d = dict(src_dict)
        momo_redirect = MomoRedirection.from_dict(d.pop("momo_redirect"))

        wallet_data_type_4 = cls(
            momo_redirect=momo_redirect,
        )

        wallet_data_type_4.additional_properties = d
        return wallet_data_type_4

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
