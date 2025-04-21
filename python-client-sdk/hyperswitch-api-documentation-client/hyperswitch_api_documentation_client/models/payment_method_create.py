from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.payment_method import PaymentMethod
from ..models.payment_method_issuer_code import PaymentMethodIssuerCode
from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.ach_bank_transfer import AchBankTransfer
    from ..models.address import Address
    from ..models.bacs_bank_transfer import BacsBankTransfer
    from ..models.card_detail import CardDetail
    from ..models.payment_method_create_data_type_0 import PaymentMethodCreateDataType0
    from ..models.payment_method_create_metadata_type_0 import PaymentMethodCreateMetadataType0
    from ..models.pix_bank_transfer import PixBankTransfer
    from ..models.sepa_bank_transfer import SepaBankTransfer
    from ..models.wallet_type_0 import WalletType0
    from ..models.wallet_type_1 import WalletType1


T = TypeVar("T", bound="PaymentMethodCreate")


@_attrs_define
class PaymentMethodCreate:
    """
    Attributes:
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
        payment_method_type (Union[None, PaymentMethodType, Unset]):
        payment_method_issuer (Union[None, Unset, str]): The name of the bank/ provider issuing the payment method to
            the end user Example: Citibank.
        payment_method_issuer_code (Union[None, PaymentMethodIssuerCode, Unset]):
        card (Union['CardDetail', None, Unset]):
        metadata (Union['PaymentMethodCreateMetadataType0', None, Unset]): You can specify up to 50 keys, with key names
            up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional,
            structured information on an object.
        customer_id (Union[None, Unset, str]): The unique identifier of the customer. Example:
            cus_y3oqhf46pyzuxjbcn2giaqnb44.
        card_network (Union[None, Unset, str]): The card network Example: Visa.
        bank_transfer (Union['AchBankTransfer', 'BacsBankTransfer', 'PixBankTransfer', 'SepaBankTransfer', None,
            Unset]):
        wallet (Union['WalletType0', 'WalletType1', None, Unset]):
        client_secret (Union[None, Unset, str]): For Client based calls, SDK will use the client_secret
            in order to call /payment_methods
            Client secret will be generated whenever a new
            payment method is created
        payment_method_data (Union['PaymentMethodCreateDataType0', None, Unset]):
        billing (Union['Address', None, Unset]):
    """

    payment_method: PaymentMethod
    payment_method_type: Union[None, PaymentMethodType, Unset] = UNSET
    payment_method_issuer: Union[None, Unset, str] = UNSET
    payment_method_issuer_code: Union[None, PaymentMethodIssuerCode, Unset] = UNSET
    card: Union["CardDetail", None, Unset] = UNSET
    metadata: Union["PaymentMethodCreateMetadataType0", None, Unset] = UNSET
    customer_id: Union[None, Unset, str] = UNSET
    card_network: Union[None, Unset, str] = UNSET
    bank_transfer: Union["AchBankTransfer", "BacsBankTransfer", "PixBankTransfer", "SepaBankTransfer", None, Unset] = (
        UNSET
    )
    wallet: Union["WalletType0", "WalletType1", None, Unset] = UNSET
    client_secret: Union[None, Unset, str] = UNSET
    payment_method_data: Union["PaymentMethodCreateDataType0", None, Unset] = UNSET
    billing: Union["Address", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.ach_bank_transfer import AchBankTransfer
        from ..models.address import Address
        from ..models.bacs_bank_transfer import BacsBankTransfer
        from ..models.card_detail import CardDetail
        from ..models.payment_method_create_data_type_0 import PaymentMethodCreateDataType0
        from ..models.payment_method_create_metadata_type_0 import PaymentMethodCreateMetadataType0
        from ..models.pix_bank_transfer import PixBankTransfer
        from ..models.sepa_bank_transfer import SepaBankTransfer
        from ..models.wallet_type_0 import WalletType0
        from ..models.wallet_type_1 import WalletType1

        payment_method = self.payment_method.value

        payment_method_type: Union[None, Unset, str]
        if isinstance(self.payment_method_type, Unset):
            payment_method_type = UNSET
        elif isinstance(self.payment_method_type, PaymentMethodType):
            payment_method_type = self.payment_method_type.value
        else:
            payment_method_type = self.payment_method_type

        payment_method_issuer: Union[None, Unset, str]
        if isinstance(self.payment_method_issuer, Unset):
            payment_method_issuer = UNSET
        else:
            payment_method_issuer = self.payment_method_issuer

        payment_method_issuer_code: Union[None, Unset, str]
        if isinstance(self.payment_method_issuer_code, Unset):
            payment_method_issuer_code = UNSET
        elif isinstance(self.payment_method_issuer_code, PaymentMethodIssuerCode):
            payment_method_issuer_code = self.payment_method_issuer_code.value
        else:
            payment_method_issuer_code = self.payment_method_issuer_code

        card: Union[None, Unset, dict[str, Any]]
        if isinstance(self.card, Unset):
            card = UNSET
        elif isinstance(self.card, CardDetail):
            card = self.card.to_dict()
        else:
            card = self.card

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, PaymentMethodCreateMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        customer_id: Union[None, Unset, str]
        if isinstance(self.customer_id, Unset):
            customer_id = UNSET
        else:
            customer_id = self.customer_id

        card_network: Union[None, Unset, str]
        if isinstance(self.card_network, Unset):
            card_network = UNSET
        else:
            card_network = self.card_network

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

        wallet: Union[None, Unset, dict[str, Any]]
        if isinstance(self.wallet, Unset):
            wallet = UNSET
        elif isinstance(self.wallet, WalletType0):
            wallet = self.wallet.to_dict()
        elif isinstance(self.wallet, WalletType1):
            wallet = self.wallet.to_dict()
        else:
            wallet = self.wallet

        client_secret: Union[None, Unset, str]
        if isinstance(self.client_secret, Unset):
            client_secret = UNSET
        else:
            client_secret = self.client_secret

        payment_method_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_method_data, Unset):
            payment_method_data = UNSET
        elif isinstance(self.payment_method_data, PaymentMethodCreateDataType0):
            payment_method_data = self.payment_method_data.to_dict()
        else:
            payment_method_data = self.payment_method_data

        billing: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing, Unset):
            billing = UNSET
        elif isinstance(self.billing, Address):
            billing = self.billing.to_dict()
        else:
            billing = self.billing

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "payment_method": payment_method,
            }
        )
        if payment_method_type is not UNSET:
            field_dict["payment_method_type"] = payment_method_type
        if payment_method_issuer is not UNSET:
            field_dict["payment_method_issuer"] = payment_method_issuer
        if payment_method_issuer_code is not UNSET:
            field_dict["payment_method_issuer_code"] = payment_method_issuer_code
        if card is not UNSET:
            field_dict["card"] = card
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if customer_id is not UNSET:
            field_dict["customer_id"] = customer_id
        if card_network is not UNSET:
            field_dict["card_network"] = card_network
        if bank_transfer is not UNSET:
            field_dict["bank_transfer"] = bank_transfer
        if wallet is not UNSET:
            field_dict["wallet"] = wallet
        if client_secret is not UNSET:
            field_dict["client_secret"] = client_secret
        if payment_method_data is not UNSET:
            field_dict["payment_method_data"] = payment_method_data
        if billing is not UNSET:
            field_dict["billing"] = billing

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.ach_bank_transfer import AchBankTransfer
        from ..models.address import Address
        from ..models.bacs_bank_transfer import BacsBankTransfer
        from ..models.card_detail import CardDetail
        from ..models.payment_method_create_data_type_0 import PaymentMethodCreateDataType0
        from ..models.payment_method_create_metadata_type_0 import PaymentMethodCreateMetadataType0
        from ..models.pix_bank_transfer import PixBankTransfer
        from ..models.sepa_bank_transfer import SepaBankTransfer
        from ..models.wallet_type_0 import WalletType0
        from ..models.wallet_type_1 import WalletType1

        d = dict(src_dict)
        payment_method = PaymentMethod(d.pop("payment_method"))

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

        def _parse_payment_method_issuer(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_method_issuer = _parse_payment_method_issuer(d.pop("payment_method_issuer", UNSET))

        def _parse_payment_method_issuer_code(data: object) -> Union[None, PaymentMethodIssuerCode, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_issuer_code_type_1 = PaymentMethodIssuerCode(data)

                return payment_method_issuer_code_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethodIssuerCode, Unset], data)

        payment_method_issuer_code = _parse_payment_method_issuer_code(d.pop("payment_method_issuer_code", UNSET))

        def _parse_card(data: object) -> Union["CardDetail", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                card_type_1 = CardDetail.from_dict(data)

                return card_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CardDetail", None, Unset], data)

        card = _parse_card(d.pop("card", UNSET))

        def _parse_metadata(data: object) -> Union["PaymentMethodCreateMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = PaymentMethodCreateMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentMethodCreateMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_customer_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        customer_id = _parse_customer_id(d.pop("customer_id", UNSET))

        def _parse_card_network(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_network = _parse_card_network(d.pop("card_network", UNSET))

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

        def _parse_wallet(data: object) -> Union["WalletType0", "WalletType1", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_wallet_type_0 = WalletType0.from_dict(data)

                return componentsschemas_wallet_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_wallet_type_1 = WalletType1.from_dict(data)

                return componentsschemas_wallet_type_1
            except:  # noqa: E722
                pass
            return cast(Union["WalletType0", "WalletType1", None, Unset], data)

        wallet = _parse_wallet(d.pop("wallet", UNSET))

        def _parse_client_secret(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_secret = _parse_client_secret(d.pop("client_secret", UNSET))

        def _parse_payment_method_data(data: object) -> Union["PaymentMethodCreateDataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_payment_method_create_data_type_0 = PaymentMethodCreateDataType0.from_dict(data)

                return componentsschemas_payment_method_create_data_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentMethodCreateDataType0", None, Unset], data)

        payment_method_data = _parse_payment_method_data(d.pop("payment_method_data", UNSET))

        def _parse_billing(data: object) -> Union["Address", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_type_1 = Address.from_dict(data)

                return billing_type_1
            except:  # noqa: E722
                pass
            return cast(Union["Address", None, Unset], data)

        billing = _parse_billing(d.pop("billing", UNSET))

        payment_method_create = cls(
            payment_method=payment_method,
            payment_method_type=payment_method_type,
            payment_method_issuer=payment_method_issuer,
            payment_method_issuer_code=payment_method_issuer_code,
            card=card,
            metadata=metadata,
            customer_id=customer_id,
            card_network=card_network,
            bank_transfer=bank_transfer,
            wallet=wallet,
            client_secret=client_secret,
            payment_method_data=payment_method_data,
            billing=billing,
        )

        return payment_method_create
