from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.touch_n_go_redirection import TouchNGoRedirection


T = TypeVar("T", bound="WalletDataType23")


@_attrs_define
class WalletDataType23:
    """
    Attributes:
        touch_n_go_redirect (TouchNGoRedirection):
    """

    touch_n_go_redirect: "TouchNGoRedirection"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        touch_n_go_redirect = self.touch_n_go_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "touch_n_go_redirect": touch_n_go_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.touch_n_go_redirection import TouchNGoRedirection

        d = dict(src_dict)
        touch_n_go_redirect = TouchNGoRedirection.from_dict(d.pop("touch_n_go_redirect"))

        wallet_data_type_23 = cls(
            touch_n_go_redirect=touch_n_go_redirect,
        )

        wallet_data_type_23.additional_properties = d
        return wallet_data_type_23

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
