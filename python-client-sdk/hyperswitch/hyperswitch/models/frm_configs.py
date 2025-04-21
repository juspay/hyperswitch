from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define

from ..models.connector_type import ConnectorType

if TYPE_CHECKING:
    from ..models.frm_payment_method import FrmPaymentMethod


T = TypeVar("T", bound="FrmConfigs")


@_attrs_define
class FrmConfigs:
    """Details of FrmConfigs are mentioned here... it should be passed in payment connector create api call, and stored in
    merchant_connector_table

        Attributes:
            gateway (ConnectorType): Type of the Connector for the financial use case. Could range from Payments to
                Accounting to Banking.
            payment_methods (list['FrmPaymentMethod']): payment methods that can be used in the payment
    """

    gateway: ConnectorType
    payment_methods: list["FrmPaymentMethod"]

    def to_dict(self) -> dict[str, Any]:
        gateway = self.gateway.value

        payment_methods = []
        for payment_methods_item_data in self.payment_methods:
            payment_methods_item = payment_methods_item_data.to_dict()
            payment_methods.append(payment_methods_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "gateway": gateway,
                "payment_methods": payment_methods,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.frm_payment_method import FrmPaymentMethod

        d = dict(src_dict)
        gateway = ConnectorType(d.pop("gateway"))

        payment_methods = []
        _payment_methods = d.pop("payment_methods")
        for payment_methods_item_data in _payment_methods:
            payment_methods_item = FrmPaymentMethod.from_dict(payment_methods_item_data)

            payment_methods.append(payment_methods_item)

        frm_configs = cls(
            gateway=gateway,
            payment_methods=payment_methods,
        )

        return frm_configs
