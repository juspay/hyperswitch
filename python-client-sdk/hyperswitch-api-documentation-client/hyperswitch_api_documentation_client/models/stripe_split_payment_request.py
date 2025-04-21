from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define

if TYPE_CHECKING:
    from ..models.payment_charge_type_type_0 import PaymentChargeTypeType0


T = TypeVar("T", bound="StripeSplitPaymentRequest")


@_attrs_define
class StripeSplitPaymentRequest:
    """Fee information for Split Payments to be charged on the payment being collected for Stripe

    Attributes:
        charge_type ('PaymentChargeTypeType0'):
        application_fees (int): Platform fees to be collected on the payment Example: 6540.
        transfer_account_id (str): Identifier for the reseller's account where the funds were transferred
    """

    charge_type: "PaymentChargeTypeType0"
    application_fees: int
    transfer_account_id: str

    def to_dict(self) -> dict[str, Any]:
        from ..models.payment_charge_type_type_0 import PaymentChargeTypeType0

        charge_type: dict[str, Any]
        if isinstance(self.charge_type, PaymentChargeTypeType0):
            charge_type = self.charge_type.to_dict()

        application_fees = self.application_fees

        transfer_account_id = self.transfer_account_id

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "charge_type": charge_type,
                "application_fees": application_fees,
                "transfer_account_id": transfer_account_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.payment_charge_type_type_0 import PaymentChargeTypeType0

        d = dict(src_dict)

        def _parse_charge_type(data: object) -> "PaymentChargeTypeType0":
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_payment_charge_type_type_0 = PaymentChargeTypeType0.from_dict(data)

            return componentsschemas_payment_charge_type_type_0

        charge_type = _parse_charge_type(d.pop("charge_type"))

        application_fees = d.pop("application_fees")

        transfer_account_id = d.pop("transfer_account_id")

        stripe_split_payment_request = cls(
            charge_type=charge_type,
            application_fees=application_fees,
            transfer_account_id=transfer_account_id,
        )

        return stripe_split_payment_request
