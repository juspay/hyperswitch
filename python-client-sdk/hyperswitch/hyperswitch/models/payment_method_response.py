import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.payment_experience import PaymentExperience
from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.ach_bank_transfer import AchBankTransfer
    from ..models.bacs_bank_transfer import BacsBankTransfer
    from ..models.card_detail_from_locker import CardDetailFromLocker
    from ..models.payment_method_response_metadata_type_0 import PaymentMethodResponseMetadataType0
    from ..models.pix_bank_transfer import PixBankTransfer
    from ..models.sepa_bank_transfer import SepaBankTransfer


T = TypeVar("T", bound="PaymentMethodResponse")


@_attrs_define
class PaymentMethodResponse:
    """
    Attributes:
        merchant_id (str): Unique identifier for a merchant Example: merchant_1671528864.
        payment_method_id (str): The unique identifier of the Payment method Example: card_rGK4Vi5iSW70MY7J2mIg.
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
        recurring_enabled (bool): Indicates whether the payment method is eligible for recurring payments Example: True.
        installment_payment_enabled (bool): Indicates whether the payment method is eligible for installment payments
            Example: True.
        customer_id (Union[None, Unset, str]): The unique identifier of the customer. Example:
            cus_y3oqhf46pyzuxjbcn2giaqnb44.
        payment_method_type (Union[None, PaymentMethodType, Unset]):
        card (Union['CardDetailFromLocker', None, Unset]):
        payment_experience (Union[None, Unset, list[PaymentExperience]]): Type of payment experience enabled with the
            connector Example: ['redirect_to_url'].
        metadata (Union['PaymentMethodResponseMetadataType0', None, Unset]): You can specify up to 50 keys, with key
            names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional,
            structured information on an object.
        created (Union[None, Unset, datetime.datetime]): A timestamp (ISO 8601 code) that determines when the payment
            method was created Example: 2023-01-18T11:04:09.922Z.
        bank_transfer (Union['AchBankTransfer', 'BacsBankTransfer', 'PixBankTransfer', 'SepaBankTransfer', None,
            Unset]):
        last_used_at (Union[None, Unset, datetime.datetime]):  Example: 2024-02-24T11:04:09.922Z.
        client_secret (Union[None, Unset, str]): For Client based calls
    """

    merchant_id: str
    payment_method_id: str
    payment_method: PaymentMethod
    recurring_enabled: bool
    installment_payment_enabled: bool
    customer_id: Union[None, Unset, str] = UNSET
    payment_method_type: Union[None, PaymentMethodType, Unset] = UNSET
    card: Union["CardDetailFromLocker", None, Unset] = UNSET
    payment_experience: Union[None, Unset, list[PaymentExperience]] = UNSET
    metadata: Union["PaymentMethodResponseMetadataType0", None, Unset] = UNSET
    created: Union[None, Unset, datetime.datetime] = UNSET
    bank_transfer: Union["AchBankTransfer", "BacsBankTransfer", "PixBankTransfer", "SepaBankTransfer", None, Unset] = (
        UNSET
    )
    last_used_at: Union[None, Unset, datetime.datetime] = UNSET
    client_secret: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.ach_bank_transfer import AchBankTransfer
        from ..models.bacs_bank_transfer import BacsBankTransfer
        from ..models.card_detail_from_locker import CardDetailFromLocker
        from ..models.payment_method_response_metadata_type_0 import PaymentMethodResponseMetadataType0
        from ..models.pix_bank_transfer import PixBankTransfer
        from ..models.sepa_bank_transfer import SepaBankTransfer

        merchant_id = self.merchant_id

        payment_method_id = self.payment_method_id

        payment_method = self.payment_method.value

        recurring_enabled = self.recurring_enabled

        installment_payment_enabled = self.installment_payment_enabled

        customer_id: Union[None, Unset, str]
        if isinstance(self.customer_id, Unset):
            customer_id = UNSET
        else:
            customer_id = self.customer_id

        payment_method_type: Union[None, Unset, str]
        if isinstance(self.payment_method_type, Unset):
            payment_method_type = UNSET
        elif isinstance(self.payment_method_type, PaymentMethodType):
            payment_method_type = self.payment_method_type.value
        else:
            payment_method_type = self.payment_method_type

        card: Union[None, Unset, dict[str, Any]]
        if isinstance(self.card, Unset):
            card = UNSET
        elif isinstance(self.card, CardDetailFromLocker):
            card = self.card.to_dict()
        else:
            card = self.card

        payment_experience: Union[None, Unset, list[str]]
        if isinstance(self.payment_experience, Unset):
            payment_experience = UNSET
        elif isinstance(self.payment_experience, list):
            payment_experience = []
            for payment_experience_type_0_item_data in self.payment_experience:
                payment_experience_type_0_item = payment_experience_type_0_item_data.value
                payment_experience.append(payment_experience_type_0_item)

        else:
            payment_experience = self.payment_experience

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, PaymentMethodResponseMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        created: Union[None, Unset, str]
        if isinstance(self.created, Unset):
            created = UNSET
        elif isinstance(self.created, datetime.datetime):
            created = self.created.isoformat()
        else:
            created = self.created

        bank_transfer: Union[None, Unset, dict[str, Any]]
        if isinstance(self.bank_transfer, Unset):
            bank_transfer = UNSET
        elif isinstance(self.bank_transfer, AchBankTransfer):
            bank_transfer = self.bank_transfer.to_dict()
        elif isinstance(self.bank_transfer, BacsBankTransfer):
            bank_transfer = self.bank_transfer.to_dict()
        elif isinstance(self.bank_transfer, SepaBankTransfer):
            bank_transfer = self.bank_transfer.to_dict()
        elif isinstance(self.bank_transfer, PixBankTransfer):
            bank_transfer = self.bank_transfer.to_dict()
        else:
            bank_transfer = self.bank_transfer

        last_used_at: Union[None, Unset, str]
        if isinstance(self.last_used_at, Unset):
            last_used_at = UNSET
        elif isinstance(self.last_used_at, datetime.datetime):
            last_used_at = self.last_used_at.isoformat()
        else:
            last_used_at = self.last_used_at

        client_secret: Union[None, Unset, str]
        if isinstance(self.client_secret, Unset):
            client_secret = UNSET
        else:
            client_secret = self.client_secret

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_id": merchant_id,
                "payment_method_id": payment_method_id,
                "payment_method": payment_method,
                "recurring_enabled": recurring_enabled,
                "installment_payment_enabled": installment_payment_enabled,
            }
        )
        if customer_id is not UNSET:
            field_dict["customer_id"] = customer_id
        if payment_method_type is not UNSET:
            field_dict["payment_method_type"] = payment_method_type
        if card is not UNSET:
            field_dict["card"] = card
        if payment_experience is not UNSET:
            field_dict["payment_experience"] = payment_experience
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if created is not UNSET:
            field_dict["created"] = created
        if bank_transfer is not UNSET:
            field_dict["bank_transfer"] = bank_transfer
        if last_used_at is not UNSET:
            field_dict["last_used_at"] = last_used_at
        if client_secret is not UNSET:
            field_dict["client_secret"] = client_secret

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.ach_bank_transfer import AchBankTransfer
        from ..models.bacs_bank_transfer import BacsBankTransfer
        from ..models.card_detail_from_locker import CardDetailFromLocker
        from ..models.payment_method_response_metadata_type_0 import PaymentMethodResponseMetadataType0
        from ..models.pix_bank_transfer import PixBankTransfer
        from ..models.sepa_bank_transfer import SepaBankTransfer

        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        payment_method_id = d.pop("payment_method_id")

        payment_method = PaymentMethod(d.pop("payment_method"))

        recurring_enabled = d.pop("recurring_enabled")

        installment_payment_enabled = d.pop("installment_payment_enabled")

        def _parse_customer_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        customer_id = _parse_customer_id(d.pop("customer_id", UNSET))

        def _parse_payment_method_type(data: object) -> Union[None, PaymentMethodType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_type_1 = PaymentMethodType(data)

                return payment_method_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethodType, Unset], data)

        payment_method_type = _parse_payment_method_type(d.pop("payment_method_type", UNSET))

        def _parse_card(data: object) -> Union["CardDetailFromLocker", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                card_type_1 = CardDetailFromLocker.from_dict(data)

                return card_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CardDetailFromLocker", None, Unset], data)

        card = _parse_card(d.pop("card", UNSET))

        def _parse_payment_experience(data: object) -> Union[None, Unset, list[PaymentExperience]]:
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
                    payment_experience_type_0_item = PaymentExperience(payment_experience_type_0_item_data)

                    payment_experience_type_0.append(payment_experience_type_0_item)

                return payment_experience_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[PaymentExperience]], data)

        payment_experience = _parse_payment_experience(d.pop("payment_experience", UNSET))

        def _parse_metadata(data: object) -> Union["PaymentMethodResponseMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = PaymentMethodResponseMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentMethodResponseMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_created(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_type_0 = isoparse(data)

                return created_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created = _parse_created(d.pop("created", UNSET))

        def _parse_bank_transfer(
            data: object,
        ) -> Union["AchBankTransfer", "BacsBankTransfer", "PixBankTransfer", "SepaBankTransfer", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_0 = AchBankTransfer.from_dict(data)

                return componentsschemas_bank_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_1 = BacsBankTransfer.from_dict(data)

                return componentsschemas_bank_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_2 = SepaBankTransfer.from_dict(data)

                return componentsschemas_bank_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_3 = PixBankTransfer.from_dict(data)

                return componentsschemas_bank_type_3
            except:  # noqa: E722
                pass
            return cast(
                Union["AchBankTransfer", "BacsBankTransfer", "PixBankTransfer", "SepaBankTransfer", None, Unset], data
            )

        bank_transfer = _parse_bank_transfer(d.pop("bank_transfer", UNSET))

        def _parse_last_used_at(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                last_used_at_type_0 = isoparse(data)

                return last_used_at_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        last_used_at = _parse_last_used_at(d.pop("last_used_at", UNSET))

        def _parse_client_secret(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_secret = _parse_client_secret(d.pop("client_secret", UNSET))

        payment_method_response = cls(
            merchant_id=merchant_id,
            payment_method_id=payment_method_id,
            payment_method=payment_method,
            recurring_enabled=recurring_enabled,
            installment_payment_enabled=installment_payment_enabled,
            customer_id=customer_id,
            payment_method_type=payment_method_type,
            card=card,
            payment_experience=payment_experience,
            metadata=metadata,
            created=created,
            bank_transfer=bank_transfer,
            last_used_at=last_used_at,
            client_secret=client_secret,
        )

        payment_method_response.additional_properties = d
        return payment_method_response

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
