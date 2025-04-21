from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

T = TypeVar("T", bound="StripeSplitRefundRequest")


@_attrs_define
class StripeSplitRefundRequest:
    """Charge specific fields for controlling the revert of funds from either platform or connected account for Stripe.
    Check sub-fields for more details.

        Attributes:
            revert_platform_fee (Union[None, Unset, bool]): Toggle for reverting the application fee that was collected for
                the payment.
                If set to false, the funds are pulled from the destination account.
            revert_transfer (Union[None, Unset, bool]): Toggle for reverting the transfer that was made during the charge.
                If set to false, the funds are pulled from the main platform's account.
    """

    revert_platform_fee: Union[None, Unset, bool] = UNSET
    revert_transfer: Union[None, Unset, bool] = UNSET

    def to_dict(self) -> dict[str, Any]:
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
        field_dict.update({})
        if revert_platform_fee is not UNSET:
            field_dict["revert_platform_fee"] = revert_platform_fee
        if revert_transfer is not UNSET:
            field_dict["revert_transfer"] = revert_transfer

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

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

        stripe_split_refund_request = cls(
            revert_platform_fee=revert_platform_fee,
            revert_transfer=revert_transfer,
        )

        return stripe_split_refund_request
