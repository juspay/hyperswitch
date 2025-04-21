from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.google_pay_third_party_sdk_data import GooglePayThirdPartySdkData


T = TypeVar("T", bound="WalletDataType14")


@_attrs_define
class WalletDataType14:
    """
    Attributes:
        google_pay_third_party_sdk (GooglePayThirdPartySdkData):
    """

    google_pay_third_party_sdk: "GooglePayThirdPartySdkData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        google_pay_third_party_sdk = self.google_pay_third_party_sdk.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "google_pay_third_party_sdk": google_pay_third_party_sdk,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.google_pay_third_party_sdk_data import GooglePayThirdPartySdkData

        d = dict(src_dict)
        google_pay_third_party_sdk = GooglePayThirdPartySdkData.from_dict(d.pop("google_pay_third_party_sdk"))

        wallet_data_type_14 = cls(
            google_pay_third_party_sdk=google_pay_third_party_sdk,
        )

        wallet_data_type_14.additional_properties = d
        return wallet_data_type_14

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
