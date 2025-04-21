from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_code_response import BankCodeResponse
    from ..models.bank_debit_types import BankDebitTypes
    from ..models.bank_transfer_types import BankTransferTypes
    from ..models.card_network_types import CardNetworkTypes
    from ..models.payment_experience_types import PaymentExperienceTypes
    from ..models.response_payment_method_types_required_fields_type_0 import (
        ResponsePaymentMethodTypesRequiredFieldsType0,
    )
    from ..models.surcharge_details_response import SurchargeDetailsResponse


T = TypeVar("T", bound="ResponsePaymentMethodTypes")


@_attrs_define
class ResponsePaymentMethodTypes:
    """
    Attributes:
        payment_method_type (PaymentMethodType): Indicates the sub type of payment method. Eg: 'google_pay' &
            'apple_pay' for wallets.
        payment_experience (Union[None, Unset, list['PaymentExperienceTypes']]): The list of payment experiences
            enabled, if applicable for a payment method type
        card_networks (Union[None, Unset, list['CardNetworkTypes']]): The list of card networks enabled, if applicable
            for a payment method type
        bank_names (Union[None, Unset, list['BankCodeResponse']]): The list of banks enabled, if applicable for a
            payment method type
        bank_debits (Union['BankDebitTypes', None, Unset]):
        bank_transfers (Union['BankTransferTypes', None, Unset]):
        required_fields (Union['ResponsePaymentMethodTypesRequiredFieldsType0', None, Unset]): Required fields for the
            payment_method_type.
        surcharge_details (Union['SurchargeDetailsResponse', None, Unset]):
        pm_auth_connector (Union[None, Unset, str]): auth service connector label for this payment method type, if
            exists
    """

    payment_method_type: PaymentMethodType
    payment_experience: Union[None, Unset, list["PaymentExperienceTypes"]] = UNSET
    card_networks: Union[None, Unset, list["CardNetworkTypes"]] = UNSET
    bank_names: Union[None, Unset, list["BankCodeResponse"]] = UNSET
    bank_debits: Union["BankDebitTypes", None, Unset] = UNSET
    bank_transfers: Union["BankTransferTypes", None, Unset] = UNSET
    required_fields: Union["ResponsePaymentMethodTypesRequiredFieldsType0", None, Unset] = UNSET
    surcharge_details: Union["SurchargeDetailsResponse", None, Unset] = UNSET
    pm_auth_connector: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.bank_debit_types import BankDebitTypes
        from ..models.bank_transfer_types import BankTransferTypes
        from ..models.response_payment_method_types_required_fields_type_0 import (
            ResponsePaymentMethodTypesRequiredFieldsType0,
        )
        from ..models.surcharge_details_response import SurchargeDetailsResponse

        payment_method_type = self.payment_method_type.value

        payment_experience: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.payment_experience, Unset):
            payment_experience = UNSET
        elif isinstance(self.payment_experience, list):
            payment_experience = []
            for payment_experience_type_0_item_data in self.payment_experience:
                payment_experience_type_0_item = payment_experience_type_0_item_data.to_dict()
                payment_experience.append(payment_experience_type_0_item)

        else:
            payment_experience = self.payment_experience

        card_networks: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.card_networks, Unset):
            card_networks = UNSET
        elif isinstance(self.card_networks, list):
            card_networks = []
            for card_networks_type_0_item_data in self.card_networks:
                card_networks_type_0_item = card_networks_type_0_item_data.to_dict()
                card_networks.append(card_networks_type_0_item)

        else:
            card_networks = self.card_networks

        bank_names: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.bank_names, Unset):
            bank_names = UNSET
        elif isinstance(self.bank_names, list):
            bank_names = []
            for bank_names_type_0_item_data in self.bank_names:
                bank_names_type_0_item = bank_names_type_0_item_data.to_dict()
                bank_names.append(bank_names_type_0_item)

        else:
            bank_names = self.bank_names

        bank_debits: Union[None, Unset, dict[str, Any]]
        if isinstance(self.bank_debits, Unset):
            bank_debits = UNSET
        elif isinstance(self.bank_debits, BankDebitTypes):
            bank_debits = self.bank_debits.to_dict()
        else:
            bank_debits = self.bank_debits

        bank_transfers: Union[None, Unset, dict[str, Any]]
        if isinstance(self.bank_transfers, Unset):
            bank_transfers = UNSET
        elif isinstance(self.bank_transfers, BankTransferTypes):
            bank_transfers = self.bank_transfers.to_dict()
        else:
            bank_transfers = self.bank_transfers

        required_fields: Union[None, Unset, dict[str, Any]]
        if isinstance(self.required_fields, Unset):
            required_fields = UNSET
        elif isinstance(self.required_fields, ResponsePaymentMethodTypesRequiredFieldsType0):
            required_fields = self.required_fields.to_dict()
        else:
            required_fields = self.required_fields

        surcharge_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.surcharge_details, Unset):
            surcharge_details = UNSET
        elif isinstance(self.surcharge_details, SurchargeDetailsResponse):
            surcharge_details = self.surcharge_details.to_dict()
        else:
            surcharge_details = self.surcharge_details

        pm_auth_connector: Union[None, Unset, str]
        if isinstance(self.pm_auth_connector, Unset):
            pm_auth_connector = UNSET
        else:
            pm_auth_connector = self.pm_auth_connector

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_method_type": payment_method_type,
            }
        )
        if payment_experience is not UNSET:
            field_dict["payment_experience"] = payment_experience
        if card_networks is not UNSET:
            field_dict["card_networks"] = card_networks
        if bank_names is not UNSET:
            field_dict["bank_names"] = bank_names
        if bank_debits is not UNSET:
            field_dict["bank_debits"] = bank_debits
        if bank_transfers is not UNSET:
            field_dict["bank_transfers"] = bank_transfers
        if required_fields is not UNSET:
            field_dict["required_fields"] = required_fields
        if surcharge_details is not UNSET:
            field_dict["surcharge_details"] = surcharge_details
        if pm_auth_connector is not UNSET:
            field_dict["pm_auth_connector"] = pm_auth_connector

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_code_response import BankCodeResponse
        from ..models.bank_debit_types import BankDebitTypes
        from ..models.bank_transfer_types import BankTransferTypes
        from ..models.card_network_types import CardNetworkTypes
        from ..models.payment_experience_types import PaymentExperienceTypes
        from ..models.response_payment_method_types_required_fields_type_0 import (
            ResponsePaymentMethodTypesRequiredFieldsType0,
        )
        from ..models.surcharge_details_response import SurchargeDetailsResponse

        d = dict(src_dict)
        payment_method_type = PaymentMethodType(d.pop("payment_method_type"))

        def _parse_payment_experience(data: object) -> Union[None, Unset, list["PaymentExperienceTypes"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                payment_experience_type_0 = []
                _payment_experience_type_0 = data
                for payment_experience_type_0_item_data in _payment_experience_type_0:
                    payment_experience_type_0_item = PaymentExperienceTypes.from_dict(
                        payment_experience_type_0_item_data
                    )

                    payment_experience_type_0.append(payment_experience_type_0_item)

                return payment_experience_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["PaymentExperienceTypes"]], data)

        payment_experience = _parse_payment_experience(d.pop("payment_experience", UNSET))

        def _parse_card_networks(data: object) -> Union[None, Unset, list["CardNetworkTypes"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                card_networks_type_0 = []
                _card_networks_type_0 = data
                for card_networks_type_0_item_data in _card_networks_type_0:
                    card_networks_type_0_item = CardNetworkTypes.from_dict(card_networks_type_0_item_data)

                    card_networks_type_0.append(card_networks_type_0_item)

                return card_networks_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["CardNetworkTypes"]], data)

        card_networks = _parse_card_networks(d.pop("card_networks", UNSET))

        def _parse_bank_names(data: object) -> Union[None, Unset, list["BankCodeResponse"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                bank_names_type_0 = []
                _bank_names_type_0 = data
                for bank_names_type_0_item_data in _bank_names_type_0:
                    bank_names_type_0_item = BankCodeResponse.from_dict(bank_names_type_0_item_data)

                    bank_names_type_0.append(bank_names_type_0_item)

                return bank_names_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["BankCodeResponse"]], data)

        bank_names = _parse_bank_names(d.pop("bank_names", UNSET))

        def _parse_bank_debits(data: object) -> Union["BankDebitTypes", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                bank_debits_type_1 = BankDebitTypes.from_dict(data)

                return bank_debits_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BankDebitTypes", None, Unset], data)

        bank_debits = _parse_bank_debits(d.pop("bank_debits", UNSET))

        def _parse_bank_transfers(data: object) -> Union["BankTransferTypes", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                bank_transfers_type_1 = BankTransferTypes.from_dict(data)

                return bank_transfers_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BankTransferTypes", None, Unset], data)

        bank_transfers = _parse_bank_transfers(d.pop("bank_transfers", UNSET))

        def _parse_required_fields(data: object) -> Union["ResponsePaymentMethodTypesRequiredFieldsType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                required_fields_type_0 = ResponsePaymentMethodTypesRequiredFieldsType0.from_dict(data)

                return required_fields_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ResponsePaymentMethodTypesRequiredFieldsType0", None, Unset], data)

        required_fields = _parse_required_fields(d.pop("required_fields", UNSET))

        def _parse_surcharge_details(data: object) -> Union["SurchargeDetailsResponse", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                surcharge_details_type_1 = SurchargeDetailsResponse.from_dict(data)

                return surcharge_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SurchargeDetailsResponse", None, Unset], data)

        surcharge_details = _parse_surcharge_details(d.pop("surcharge_details", UNSET))

        def _parse_pm_auth_connector(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        pm_auth_connector = _parse_pm_auth_connector(d.pop("pm_auth_connector", UNSET))

        response_payment_method_types = cls(
            payment_method_type=payment_method_type,
            payment_experience=payment_experience,
            card_networks=card_networks,
            bank_names=bank_names,
            bank_debits=bank_debits,
            bank_transfers=bank_transfers,
            required_fields=required_fields,
            surcharge_details=surcharge_details,
            pm_auth_connector=pm_auth_connector,
        )

        response_payment_method_types.additional_properties = d
        return response_payment_method_types

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
