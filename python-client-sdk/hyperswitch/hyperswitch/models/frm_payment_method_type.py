from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define

from ..models.card_network import CardNetwork
from ..models.frm_action import FrmAction
from ..models.frm_preferred_flow_types import FrmPreferredFlowTypes
from ..models.payment_method_type import PaymentMethodType

T = TypeVar("T", bound="FrmPaymentMethodType")


@_attrs_define
class FrmPaymentMethodType:
    """Details of FrmPaymentMethodType are mentioned here... it should be passed in payment connector create api call, and
    stored in merchant_connector_table

        Attributes:
            payment_method_type (PaymentMethodType): Indicates the sub type of payment method. Eg: 'google_pay' &
                'apple_pay' for wallets.
            card_networks (CardNetwork): Indicates the card network.
            flow (FrmPreferredFlowTypes):
            action (FrmAction):
    """

    payment_method_type: PaymentMethodType
    card_networks: CardNetwork
    flow: FrmPreferredFlowTypes
    action: FrmAction

    def to_dict(self) -> dict[str, Any]:
        payment_method_type = self.payment_method_type.value

        card_networks = self.card_networks.value

        flow = self.flow.value

        action = self.action.value

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "payment_method_type": payment_method_type,
                "card_networks": card_networks,
                "flow": flow,
                "action": action,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payment_method_type = PaymentMethodType(d.pop("payment_method_type"))

        card_networks = CardNetwork(d.pop("card_networks"))

        flow = FrmPreferredFlowTypes(d.pop("flow"))

        action = FrmAction(d.pop("action"))

        frm_payment_method_type = cls(
            payment_method_type=payment_method_type,
            card_networks=card_networks,
            flow=flow,
            action=action,
        )

        return frm_payment_method_type
