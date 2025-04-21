from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..models.payment_type import PaymentType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.mandate_type_type_0 import MandateTypeType0
    from ..models.mandate_type_type_1 import MandateTypeType1
    from ..models.response_payment_methods_enabled import ResponsePaymentMethodsEnabled


T = TypeVar("T", bound="PaymentMethodListResponse")


@_attrs_define
class PaymentMethodListResponse:
    """
    Attributes:
        currency (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States Dollar.
        payment_methods (list['ResponsePaymentMethodsEnabled']): Information about the payment method
        mandate_payment (Union['MandateTypeType0', 'MandateTypeType1']):
        show_surcharge_breakup_screen (bool): flag to indicate if surcharge and tax breakup screen should be shown or
            not
        request_external_three_ds_authentication (bool): flag to indicate whether to perform external 3ds authentication
            Example: True.
        is_tax_calculation_enabled (bool): flag that indicates whether to calculate tax on the order amount
        redirect_url (Union[None, Unset, str]): Redirect URL of the merchant Example: https://www.google.com.
        merchant_name (Union[None, Unset, str]):
        payment_type (Union[None, PaymentType, Unset]):
        collect_shipping_details_from_wallets (Union[None, Unset, bool]): flag that indicates whether to collect
            shipping details from wallets or from the customer
        collect_billing_details_from_wallets (Union[None, Unset, bool]): flag that indicates whether to collect billing
            details from wallets or from the customer
    """

    currency: Currency
    payment_methods: list["ResponsePaymentMethodsEnabled"]
    mandate_payment: Union["MandateTypeType0", "MandateTypeType1"]
    show_surcharge_breakup_screen: bool
    request_external_three_ds_authentication: bool
    is_tax_calculation_enabled: bool
    redirect_url: Union[None, Unset, str] = UNSET
    merchant_name: Union[None, Unset, str] = UNSET
    payment_type: Union[None, PaymentType, Unset] = UNSET
    collect_shipping_details_from_wallets: Union[None, Unset, bool] = UNSET
    collect_billing_details_from_wallets: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.mandate_type_type_0 import MandateTypeType0

        currency = self.currency.value

        payment_methods = []
        for payment_methods_item_data in self.payment_methods:
            payment_methods_item = payment_methods_item_data.to_dict()
            payment_methods.append(payment_methods_item)

        mandate_payment: dict[str, Any]
        if isinstance(self.mandate_payment, MandateTypeType0):
            mandate_payment = self.mandate_payment.to_dict()
        else:
            mandate_payment = self.mandate_payment.to_dict()

        show_surcharge_breakup_screen = self.show_surcharge_breakup_screen

        request_external_three_ds_authentication = self.request_external_three_ds_authentication

        is_tax_calculation_enabled = self.is_tax_calculation_enabled

        redirect_url: Union[None, Unset, str]
        if isinstance(self.redirect_url, Unset):
            redirect_url = UNSET
        else:
            redirect_url = self.redirect_url

        merchant_name: Union[None, Unset, str]
        if isinstance(self.merchant_name, Unset):
            merchant_name = UNSET
        else:
            merchant_name = self.merchant_name

        payment_type: Union[None, Unset, str]
        if isinstance(self.payment_type, Unset):
            payment_type = UNSET
        elif isinstance(self.payment_type, PaymentType):
            payment_type = self.payment_type.value
        else:
            payment_type = self.payment_type

        collect_shipping_details_from_wallets: Union[None, Unset, bool]
        if isinstance(self.collect_shipping_details_from_wallets, Unset):
            collect_shipping_details_from_wallets = UNSET
        else:
            collect_shipping_details_from_wallets = self.collect_shipping_details_from_wallets

        collect_billing_details_from_wallets: Union[None, Unset, bool]
        if isinstance(self.collect_billing_details_from_wallets, Unset):
            collect_billing_details_from_wallets = UNSET
        else:
            collect_billing_details_from_wallets = self.collect_billing_details_from_wallets

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "currency": currency,
                "payment_methods": payment_methods,
                "mandate_payment": mandate_payment,
                "show_surcharge_breakup_screen": show_surcharge_breakup_screen,
                "request_external_three_ds_authentication": request_external_three_ds_authentication,
                "is_tax_calculation_enabled": is_tax_calculation_enabled,
            }
        )
        if redirect_url is not UNSET:
            field_dict["redirect_url"] = redirect_url
        if merchant_name is not UNSET:
            field_dict["merchant_name"] = merchant_name
        if payment_type is not UNSET:
            field_dict["payment_type"] = payment_type
        if collect_shipping_details_from_wallets is not UNSET:
            field_dict["collect_shipping_details_from_wallets"] = collect_shipping_details_from_wallets
        if collect_billing_details_from_wallets is not UNSET:
            field_dict["collect_billing_details_from_wallets"] = collect_billing_details_from_wallets

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.mandate_type_type_0 import MandateTypeType0
        from ..models.mandate_type_type_1 import MandateTypeType1
        from ..models.response_payment_methods_enabled import ResponsePaymentMethodsEnabled

        d = dict(src_dict)
        currency = Currency(d.pop("currency"))

        payment_methods = []
        _payment_methods = d.pop("payment_methods")
        for payment_methods_item_data in _payment_methods:
            payment_methods_item = ResponsePaymentMethodsEnabled.from_dict(payment_methods_item_data)

            payment_methods.append(payment_methods_item)

        def _parse_mandate_payment(data: object) -> Union["MandateTypeType0", "MandateTypeType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_mandate_type_type_0 = MandateTypeType0.from_dict(data)

                return componentsschemas_mandate_type_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_mandate_type_type_1 = MandateTypeType1.from_dict(data)

            return componentsschemas_mandate_type_type_1

        mandate_payment = _parse_mandate_payment(d.pop("mandate_payment"))

        show_surcharge_breakup_screen = d.pop("show_surcharge_breakup_screen")

        request_external_three_ds_authentication = d.pop("request_external_three_ds_authentication")

        is_tax_calculation_enabled = d.pop("is_tax_calculation_enabled")

        def _parse_redirect_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        redirect_url = _parse_redirect_url(d.pop("redirect_url", UNSET))

        def _parse_merchant_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_name = _parse_merchant_name(d.pop("merchant_name", UNSET))

        def _parse_payment_type(data: object) -> Union[None, PaymentType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_type_type_1 = PaymentType(data)

                return payment_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentType, Unset], data)

        payment_type = _parse_payment_type(d.pop("payment_type", UNSET))

        def _parse_collect_shipping_details_from_wallets(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        collect_shipping_details_from_wallets = _parse_collect_shipping_details_from_wallets(
            d.pop("collect_shipping_details_from_wallets", UNSET)
        )

        def _parse_collect_billing_details_from_wallets(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        collect_billing_details_from_wallets = _parse_collect_billing_details_from_wallets(
            d.pop("collect_billing_details_from_wallets", UNSET)
        )

        payment_method_list_response = cls(
            currency=currency,
            payment_methods=payment_methods,
            mandate_payment=mandate_payment,
            show_surcharge_breakup_screen=show_surcharge_breakup_screen,
            request_external_three_ds_authentication=request_external_three_ds_authentication,
            is_tax_calculation_enabled=is_tax_calculation_enabled,
            redirect_url=redirect_url,
            merchant_name=merchant_name,
            payment_type=payment_type,
            collect_shipping_details_from_wallets=collect_shipping_details_from_wallets,
            collect_billing_details_from_wallets=collect_billing_details_from_wallets,
        )

        payment_method_list_response.additional_properties = d
        return payment_method_list_response

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
