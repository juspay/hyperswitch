from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.adyen_split_type import AdyenSplitType
from ..types import UNSET, Unset

T = TypeVar("T", bound="AdyenSplitItem")


@_attrs_define
class AdyenSplitItem:
    """Data for the split items

    Attributes:
        amount (int): The amount of the split item Example: 6540.
        split_type (AdyenSplitType):
        reference (str): Unique Identifier for the split item
        account (Union[None, Unset, str]): The unique identifier of the account to which the split amount is allocated.
        description (Union[None, Unset, str]): Description for the part of the payment that will be allocated to the
            specified account.
    """

    amount: int
    split_type: AdyenSplitType
    reference: str
    account: Union[None, Unset, str] = UNSET
    description: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        amount = self.amount

        split_type = self.split_type.value

        reference = self.reference

        account: Union[None, Unset, str]
        if isinstance(self.account, Unset):
            account = UNSET
        else:
            account = self.account

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "amount": amount,
                "split_type": split_type,
                "reference": reference,
            }
        )
        if account is not UNSET:
            field_dict["account"] = account
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        amount = d.pop("amount")

        split_type = AdyenSplitType(d.pop("split_type"))

        reference = d.pop("reference")

        def _parse_account(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        account = _parse_account(d.pop("account", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        adyen_split_item = cls(
            amount=amount,
            split_type=split_type,
            reference=reference,
            account=account,
            description=description,
        )

        return adyen_split_item
