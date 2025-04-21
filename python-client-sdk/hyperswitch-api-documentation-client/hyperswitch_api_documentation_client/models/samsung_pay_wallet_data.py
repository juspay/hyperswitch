from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.samsung_pay_app_wallet_data import SamsungPayAppWalletData
    from ..models.samsung_pay_web_wallet_data import SamsungPayWebWalletData


T = TypeVar("T", bound="SamsungPayWalletData")


@_attrs_define
class SamsungPayWalletData:
    """
    Attributes:
        payment_credential (Union['SamsungPayAppWalletData', 'SamsungPayWebWalletData']):
    """

    payment_credential: Union["SamsungPayAppWalletData", "SamsungPayWebWalletData"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.samsung_pay_web_wallet_data import SamsungPayWebWalletData

        payment_credential: dict[str, Any]
        if isinstance(self.payment_credential, SamsungPayWebWalletData):
            payment_credential = self.payment_credential.to_dict()
        else:
            payment_credential = self.payment_credential.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_credential": payment_credential,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.samsung_pay_app_wallet_data import SamsungPayAppWalletData
        from ..models.samsung_pay_web_wallet_data import SamsungPayWebWalletData

        d = dict(src_dict)

        def _parse_payment_credential(data: object) -> Union["SamsungPayAppWalletData", "SamsungPayWebWalletData"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_samsung_pay_wallet_credentials_type_0 = SamsungPayWebWalletData.from_dict(data)

                return componentsschemas_samsung_pay_wallet_credentials_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_samsung_pay_wallet_credentials_type_1 = SamsungPayAppWalletData.from_dict(data)

            return componentsschemas_samsung_pay_wallet_credentials_type_1

        payment_credential = _parse_payment_credential(d.pop("payment_credential"))

        samsung_pay_wallet_data = cls(
            payment_credential=payment_credential,
        )

        samsung_pay_wallet_data.additional_properties = d
        return samsung_pay_wallet_data

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
