from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap


T = TypeVar("T", bound="PaymentsCancelRequest")


@_attrs_define
class PaymentsCancelRequest:
    """
    Attributes:
        cancellation_reason (Union[None, Unset, str]): The reason for the payment cancel
        merchant_connector_details (Union['MerchantConnectorDetailsWrap', None, Unset]):
    """

    cancellation_reason: Union[None, Unset, str] = UNSET
    merchant_connector_details: Union["MerchantConnectorDetailsWrap", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        cancellation_reason: Union[None, Unset, str]
        if isinstance(self.cancellation_reason, Unset):
            cancellation_reason = UNSET
        else:
            cancellation_reason = self.cancellation_reason

        merchant_connector_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_connector_details, Unset):
            merchant_connector_details = UNSET
        elif isinstance(self.merchant_connector_details, MerchantConnectorDetailsWrap):
            merchant_connector_details = self.merchant_connector_details.to_dict()
        else:
            merchant_connector_details = self.merchant_connector_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if cancellation_reason is not UNSET:
            field_dict["cancellation_reason"] = cancellation_reason
        if merchant_connector_details is not UNSET:
            field_dict["merchant_connector_details"] = merchant_connector_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        d = dict(src_dict)

        def _parse_cancellation_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        cancellation_reason = _parse_cancellation_reason(d.pop("cancellation_reason", UNSET))

        def _parse_merchant_connector_details(data: object) -> Union["MerchantConnectorDetailsWrap", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_connector_details_type_1 = MerchantConnectorDetailsWrap.from_dict(data)

                return merchant_connector_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsWrap", None, Unset], data)

        merchant_connector_details = _parse_merchant_connector_details(d.pop("merchant_connector_details", UNSET))

        payments_cancel_request = cls(
            cancellation_reason=cancellation_reason,
            merchant_connector_details=merchant_connector_details,
        )

        payments_cancel_request.additional_properties = d
        return payments_cancel_request

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
