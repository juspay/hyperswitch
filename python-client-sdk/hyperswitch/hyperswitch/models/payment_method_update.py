from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.card_detail_update import CardDetailUpdate


T = TypeVar("T", bound="PaymentMethodUpdate")


@_attrs_define
class PaymentMethodUpdate:
    """
    Attributes:
        card (Union['CardDetailUpdate', None, Unset]):
        client_secret (Union[None, Unset, str]): This is a 15 minute expiry token which shall be used from the client to
            authenticate and perform sessions from the SDK Example: secret_k2uj3he2893eiu2d.
    """

    card: Union["CardDetailUpdate", None, Unset] = UNSET
    client_secret: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.card_detail_update import CardDetailUpdate

        card: Union[None, Unset, dict[str, Any]]
        if isinstance(self.card, Unset):
            card = UNSET
        elif isinstance(self.card, CardDetailUpdate):
            card = self.card.to_dict()
        else:
            card = self.card

        client_secret: Union[None, Unset, str]
        if isinstance(self.client_secret, Unset):
            client_secret = UNSET
        else:
            client_secret = self.client_secret

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if card is not UNSET:
            field_dict["card"] = card
        if client_secret is not UNSET:
            field_dict["client_secret"] = client_secret

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.card_detail_update import CardDetailUpdate

        d = dict(src_dict)

        def _parse_card(data: object) -> Union["CardDetailUpdate", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                card_type_1 = CardDetailUpdate.from_dict(data)

                return card_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CardDetailUpdate", None, Unset], data)

        card = _parse_card(d.pop("card", UNSET))

        def _parse_client_secret(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_secret = _parse_client_secret(d.pop("client_secret", UNSET))

        payment_method_update = cls(
            card=card,
            client_secret=client_secret,
        )

        return payment_method_update
