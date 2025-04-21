from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.payment_charge_type_type_0 import PaymentChargeTypeType0


T = TypeVar("T", bound="StripeChargeResponseData")


@_attrs_define
class StripeChargeResponseData:
    """Fee information to be charged on the payment being collected via Stripe

    Attributes:
        charge_type ('PaymentChargeTypeType0'):
        application_fees (int): Platform fees collected on the payment Example: 6540.
        transfer_account_id (str): Identifier for the reseller's account where the funds were transferred
        charge_id (Union[None, Unset, str]): Identifier for charge created for the payment
    """

    charge_type: "PaymentChargeTypeType0"
    application_fees: int
    transfer_account_id: str
    charge_id: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.payment_charge_type_type_0 import PaymentChargeTypeType0

        charge_type: dict[str, Any]
        if isinstance(self.charge_type, PaymentChargeTypeType0):
            charge_type = self.charge_type.to_dict()

        application_fees = self.application_fees

        transfer_account_id = self.transfer_account_id

        charge_id: Union[None, Unset, str]
        if isinstance(self.charge_id, Unset):
            charge_id = UNSET
        else:
            charge_id = self.charge_id

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "charge_type": charge_type,
                "application_fees": application_fees,
                "transfer_account_id": transfer_account_id,
            }
        )
        if charge_id is not UNSET:
            field_dict["charge_id"] = charge_id

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

        def _parse_charge_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        charge_id = _parse_charge_id(d.pop("charge_id", UNSET))

        stripe_charge_response_data = cls(
            charge_type=charge_type,
            application_fees=application_fees,
            transfer_account_id=transfer_account_id,
            charge_id=charge_id,
        )

        return stripe_charge_response_data
