from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.samsung_pay_protocol_type import SamsungPayProtocolType

if TYPE_CHECKING:
    from ..models.samsung_pay_amount_details import SamsungPayAmountDetails
    from ..models.samsung_pay_merchant_payment_information import SamsungPayMerchantPaymentInformation


T = TypeVar("T", bound="SamsungPaySessionTokenResponse")


@_attrs_define
class SamsungPaySessionTokenResponse:
    """
    Attributes:
        version (str): Samsung Pay API version
        service_id (str): Samsung Pay service ID to which session call needs to be made
        order_number (str): Order number of the transaction
        merchant (SamsungPayMerchantPaymentInformation):
        amount (SamsungPayAmountDetails):
        protocol (SamsungPayProtocolType):
        allowed_brands (list[str]): List of supported card brands
        billing_address_required (bool): Is billing address required to be collected from wallet
        shipping_address_required (bool): Is shipping address required to be collected from wallet
    """

    version: str
    service_id: str
    order_number: str
    merchant: "SamsungPayMerchantPaymentInformation"
    amount: "SamsungPayAmountDetails"
    protocol: SamsungPayProtocolType
    allowed_brands: list[str]
    billing_address_required: bool
    shipping_address_required: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        version = self.version

        service_id = self.service_id

        order_number = self.order_number

        merchant = self.merchant.to_dict()

        amount = self.amount.to_dict()

        protocol = self.protocol.value

        allowed_brands = self.allowed_brands

        billing_address_required = self.billing_address_required

        shipping_address_required = self.shipping_address_required

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "version": version,
                "service_id": service_id,
                "order_number": order_number,
                "merchant": merchant,
                "amount": amount,
                "protocol": protocol,
                "allowed_brands": allowed_brands,
                "billing_address_required": billing_address_required,
                "shipping_address_required": shipping_address_required,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.samsung_pay_amount_details import SamsungPayAmountDetails
        from ..models.samsung_pay_merchant_payment_information import SamsungPayMerchantPaymentInformation

        d = dict(src_dict)
        version = d.pop("version")

        service_id = d.pop("service_id")

        order_number = d.pop("order_number")

        merchant = SamsungPayMerchantPaymentInformation.from_dict(d.pop("merchant"))

        amount = SamsungPayAmountDetails.from_dict(d.pop("amount"))

        protocol = SamsungPayProtocolType(d.pop("protocol"))

        allowed_brands = cast(list[str], d.pop("allowed_brands"))

        billing_address_required = d.pop("billing_address_required")

        shipping_address_required = d.pop("shipping_address_required")

        samsung_pay_session_token_response = cls(
            version=version,
            service_id=service_id,
            order_number=order_number,
            merchant=merchant,
            amount=amount,
            protocol=protocol,
            allowed_brands=allowed_brands,
            billing_address_required=billing_address_required,
            shipping_address_required=shipping_address_required,
        )

        samsung_pay_session_token_response.additional_properties = d
        return samsung_pay_session_token_response

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
