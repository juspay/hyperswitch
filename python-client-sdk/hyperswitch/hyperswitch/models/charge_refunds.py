from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="ChargeRefunds")


@_attrs_define
class ChargeRefunds:
    """Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-
    fields for more details.

        Attributes:
            charge_id (str): Identifier for charge created for the payment
            revert_platform_fee (Union[None, Unset, bool]): Toggle for reverting the application fee that was collected for
                the payment.
                If set to false, the funds are pulled from the destination account.
            revert_transfer (Union[None, Unset, bool]): Toggle for reverting the transfer that was made during the charge.
                If set to false, the funds are pulled from the main platform's account.
    """

    charge_id: str
    revert_platform_fee: Union[None, Unset, bool] = UNSET
    revert_transfer: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        charge_id = self.charge_id

        revert_platform_fee: Union[None, Unset, bool]
        if isinstance(self.revert_platform_fee, Unset):
            revert_platform_fee = UNSET
        else:
            revert_platform_fee = self.revert_platform_fee

        revert_transfer: Union[None, Unset, bool]
        if isinstance(self.revert_transfer, Unset):
            revert_transfer = UNSET
        else:
            revert_transfer = self.revert_transfer

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "charge_id": charge_id,
            }
        )
        if revert_platform_fee is not UNSET:
            field_dict["revert_platform_fee"] = revert_platform_fee
        if revert_transfer is not UNSET:
            field_dict["revert_transfer"] = revert_transfer

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        charge_id = d.pop("charge_id")

        def _parse_revert_platform_fee(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        revert_platform_fee = _parse_revert_platform_fee(d.pop("revert_platform_fee", UNSET))

        def _parse_revert_transfer(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        revert_transfer = _parse_revert_transfer(d.pop("revert_transfer", UNSET))

        charge_refunds = cls(
            charge_id=charge_id,
            revert_platform_fee=revert_platform_fee,
            revert_transfer=revert_transfer,
        )

        charge_refunds.additional_properties = d
        return charge_refunds

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
