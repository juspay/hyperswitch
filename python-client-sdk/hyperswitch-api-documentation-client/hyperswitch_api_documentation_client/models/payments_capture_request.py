from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap


T = TypeVar("T", bound="PaymentsCaptureRequest")


@_attrs_define
class PaymentsCaptureRequest:
    """
    Attributes:
        amount_to_capture (int): The Amount to be captured/ debited from the user's payment method. If not passed the
            full amount will be captured. Example: 6540.
        merchant_id (Union[None, Unset, str]): The unique identifier for the merchant
        refund_uncaptured_amount (Union[None, Unset, bool]): Decider to refund the uncaptured amount
        statement_descriptor_suffix (Union[None, Unset, str]): Provides information about a card payment that customers
            see on their statements.
        statement_descriptor_prefix (Union[None, Unset, str]): Concatenated with the statement descriptor suffix that’s
            set on the account to form the complete statement descriptor.
        merchant_connector_details (Union['MerchantConnectorDetailsWrap', None, Unset]):
    """

    amount_to_capture: int
    merchant_id: Union[None, Unset, str] = UNSET
    refund_uncaptured_amount: Union[None, Unset, bool] = UNSET
    statement_descriptor_suffix: Union[None, Unset, str] = UNSET
    statement_descriptor_prefix: Union[None, Unset, str] = UNSET
    merchant_connector_details: Union["MerchantConnectorDetailsWrap", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        amount_to_capture = self.amount_to_capture

        merchant_id: Union[None, Unset, str]
        if isinstance(self.merchant_id, Unset):
            merchant_id = UNSET
        else:
            merchant_id = self.merchant_id

        refund_uncaptured_amount: Union[None, Unset, bool]
        if isinstance(self.refund_uncaptured_amount, Unset):
            refund_uncaptured_amount = UNSET
        else:
            refund_uncaptured_amount = self.refund_uncaptured_amount

        statement_descriptor_suffix: Union[None, Unset, str]
        if isinstance(self.statement_descriptor_suffix, Unset):
            statement_descriptor_suffix = UNSET
        else:
            statement_descriptor_suffix = self.statement_descriptor_suffix

        statement_descriptor_prefix: Union[None, Unset, str]
        if isinstance(self.statement_descriptor_prefix, Unset):
            statement_descriptor_prefix = UNSET
        else:
            statement_descriptor_prefix = self.statement_descriptor_prefix

        merchant_connector_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_connector_details, Unset):
            merchant_connector_details = UNSET
        elif isinstance(self.merchant_connector_details, MerchantConnectorDetailsWrap):
            merchant_connector_details = self.merchant_connector_details.to_dict()
        else:
            merchant_connector_details = self.merchant_connector_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "amount_to_capture": amount_to_capture,
            }
        )
        if merchant_id is not UNSET:
            field_dict["merchant_id"] = merchant_id
        if refund_uncaptured_amount is not UNSET:
            field_dict["refund_uncaptured_amount"] = refund_uncaptured_amount
        if statement_descriptor_suffix is not UNSET:
            field_dict["statement_descriptor_suffix"] = statement_descriptor_suffix
        if statement_descriptor_prefix is not UNSET:
            field_dict["statement_descriptor_prefix"] = statement_descriptor_prefix
        if merchant_connector_details is not UNSET:
            field_dict["merchant_connector_details"] = merchant_connector_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        d = dict(src_dict)
        amount_to_capture = d.pop("amount_to_capture")

        def _parse_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_id = _parse_merchant_id(d.pop("merchant_id", UNSET))

        def _parse_refund_uncaptured_amount(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        refund_uncaptured_amount = _parse_refund_uncaptured_amount(d.pop("refund_uncaptured_amount", UNSET))

        def _parse_statement_descriptor_suffix(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        statement_descriptor_suffix = _parse_statement_descriptor_suffix(d.pop("statement_descriptor_suffix", UNSET))

        def _parse_statement_descriptor_prefix(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        statement_descriptor_prefix = _parse_statement_descriptor_prefix(d.pop("statement_descriptor_prefix", UNSET))

        def _parse_merchant_connector_details(data: object) -> Union["MerchantConnectorDetailsWrap", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_connector_details_type_1 = MerchantConnectorDetailsWrap.from_dict(data)

                return merchant_connector_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsWrap", None, Unset], data)

        merchant_connector_details = _parse_merchant_connector_details(d.pop("merchant_connector_details", UNSET))

        payments_capture_request = cls(
            amount_to_capture=amount_to_capture,
            merchant_id=merchant_id,
            refund_uncaptured_amount=refund_uncaptured_amount,
            statement_descriptor_suffix=statement_descriptor_suffix,
            statement_descriptor_prefix=statement_descriptor_prefix,
            merchant_connector_details=merchant_connector_details,
        )

        payments_capture_request.additional_properties = d
        return payments_capture_request

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
