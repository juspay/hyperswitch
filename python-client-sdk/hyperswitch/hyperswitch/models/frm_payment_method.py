from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.frm_preferred_flow_types import FrmPreferredFlowTypes
from ..models.payment_method import PaymentMethod
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.frm_payment_method_type import FrmPaymentMethodType


T = TypeVar("T", bound="FrmPaymentMethod")


@_attrs_define
class FrmPaymentMethod:
    """Details of FrmPaymentMethod are mentioned here... it should be passed in payment connector create api call, and
    stored in merchant_connector_table

        Attributes:
            payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
            payment_method_types (Union[None, Unset, list['FrmPaymentMethodType']]): payment method types(credit, debit)
                that can be used in the payment. This field is deprecated. It has not been removed to provide backward
                compatibility.
            flow (Union[FrmPreferredFlowTypes, None, Unset]):
    """

    payment_method: PaymentMethod
    payment_method_types: Union[None, Unset, list["FrmPaymentMethodType"]] = UNSET
    flow: Union[FrmPreferredFlowTypes, None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        payment_method = self.payment_method.value

        payment_method_types: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.payment_method_types, Unset):
            payment_method_types = UNSET
        elif isinstance(self.payment_method_types, list):
            payment_method_types = []
            for payment_method_types_type_0_item_data in self.payment_method_types:
                payment_method_types_type_0_item = payment_method_types_type_0_item_data.to_dict()
                payment_method_types.append(payment_method_types_type_0_item)

        else:
            payment_method_types = self.payment_method_types

        flow: Union[None, Unset, str]
        if isinstance(self.flow, Unset):
            flow = UNSET
        elif isinstance(self.flow, FrmPreferredFlowTypes):
            flow = self.flow.value
        else:
            flow = self.flow

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "payment_method": payment_method,
            }
        )
        if payment_method_types is not UNSET:
            field_dict["payment_method_types"] = payment_method_types
        if flow is not UNSET:
            field_dict["flow"] = flow

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.frm_payment_method_type import FrmPaymentMethodType

        d = dict(src_dict)
        payment_method = PaymentMethod(d.pop("payment_method"))

        def _parse_payment_method_types(data: object) -> Union[None, Unset, list["FrmPaymentMethodType"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                payment_method_types_type_0 = []
                _payment_method_types_type_0 = data
                for payment_method_types_type_0_item_data in _payment_method_types_type_0:
                    payment_method_types_type_0_item = FrmPaymentMethodType.from_dict(
                        payment_method_types_type_0_item_data
                    )

                    payment_method_types_type_0.append(payment_method_types_type_0_item)

                return payment_method_types_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["FrmPaymentMethodType"]], data)

        payment_method_types = _parse_payment_method_types(d.pop("payment_method_types", UNSET))

        def _parse_flow(data: object) -> Union[FrmPreferredFlowTypes, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                flow_type_1 = FrmPreferredFlowTypes(data)

                return flow_type_1
            except:  # noqa: E722
                pass
            return cast(Union[FrmPreferredFlowTypes, None, Unset], data)

        flow = _parse_flow(d.pop("flow", UNSET))

        frm_payment_method = cls(
            payment_method=payment_method,
            payment_method_types=payment_method_types,
            flow=flow,
        )

        return frm_payment_method
