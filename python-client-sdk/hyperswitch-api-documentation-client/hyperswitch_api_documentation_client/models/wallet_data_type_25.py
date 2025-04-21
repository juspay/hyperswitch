from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.we_chat_pay_qr import WeChatPayQr


T = TypeVar("T", bound="WalletDataType25")


@_attrs_define
class WalletDataType25:
    """
    Attributes:
        we_chat_pay_qr (WeChatPayQr):
    """

    we_chat_pay_qr: "WeChatPayQr"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        we_chat_pay_qr = self.we_chat_pay_qr.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "we_chat_pay_qr": we_chat_pay_qr,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.we_chat_pay_qr import WeChatPayQr

        d = dict(src_dict)
        we_chat_pay_qr = WeChatPayQr.from_dict(d.pop("we_chat_pay_qr"))

        wallet_data_type_25 = cls(
            we_chat_pay_qr=we_chat_pay_qr,
        )

        wallet_data_type_25.additional_properties = d
        return wallet_data_type_25

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
