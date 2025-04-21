from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.paypal_additional_data import PaypalAdditionalData
    from ..models.venmo_additional_data import VenmoAdditionalData


T = TypeVar("T", bound="PayoutMethodDataResponseType2")


@_attrs_define
class PayoutMethodDataResponseType2:
    """
    Attributes:
        wallet (Union['PaypalAdditionalData', 'VenmoAdditionalData']): Masked payout method details for wallet payout
            method
    """

    wallet: Union["PaypalAdditionalData", "VenmoAdditionalData"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.paypal_additional_data import PaypalAdditionalData

        wallet: dict[str, Any]
        if isinstance(self.wallet, PaypalAdditionalData):
            wallet = self.wallet.to_dict()
        else:
            wallet = self.wallet.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "wallet": wallet,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.paypal_additional_data import PaypalAdditionalData
        from ..models.venmo_additional_data import VenmoAdditionalData

        d = dict(src_dict)

        def _parse_wallet(data: object) -> Union["PaypalAdditionalData", "VenmoAdditionalData"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_wallet_additional_data_type_0 = PaypalAdditionalData.from_dict(data)

                return componentsschemas_wallet_additional_data_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_wallet_additional_data_type_1 = VenmoAdditionalData.from_dict(data)

            return componentsschemas_wallet_additional_data_type_1

        wallet = _parse_wallet(d.pop("wallet"))

        payout_method_data_response_type_2 = cls(
            wallet=wallet,
        )

        payout_method_data_response_type_2.additional_properties = d
        return payout_method_data_response_type_2

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
