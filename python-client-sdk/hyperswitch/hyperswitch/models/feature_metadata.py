from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.apple_pay_recurring_details import ApplePayRecurringDetails
    from ..models.redirect_response import RedirectResponse


T = TypeVar("T", bound="FeatureMetadata")


@_attrs_define
class FeatureMetadata:
    """additional data that might be required by hyperswitch

    Attributes:
        redirect_response (Union['RedirectResponse', None, Unset]):
        search_tags (Union[None, Unset, list[str]]): Additional tags to be used for global search
        apple_pay_recurring_details (Union['ApplePayRecurringDetails', None, Unset]):
    """

    redirect_response: Union["RedirectResponse", None, Unset] = UNSET
    search_tags: Union[None, Unset, list[str]] = UNSET
    apple_pay_recurring_details: Union["ApplePayRecurringDetails", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.apple_pay_recurring_details import ApplePayRecurringDetails
        from ..models.redirect_response import RedirectResponse

        redirect_response: Union[None, Unset, dict[str, Any]]
        if isinstance(self.redirect_response, Unset):
            redirect_response = UNSET
        elif isinstance(self.redirect_response, RedirectResponse):
            redirect_response = self.redirect_response.to_dict()
        else:
            redirect_response = self.redirect_response

        search_tags: Union[None, Unset, list[str]]
        if isinstance(self.search_tags, Unset):
            search_tags = UNSET
        elif isinstance(self.search_tags, list):
            search_tags = self.search_tags

        else:
            search_tags = self.search_tags

        apple_pay_recurring_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.apple_pay_recurring_details, Unset):
            apple_pay_recurring_details = UNSET
        elif isinstance(self.apple_pay_recurring_details, ApplePayRecurringDetails):
            apple_pay_recurring_details = self.apple_pay_recurring_details.to_dict()
        else:
            apple_pay_recurring_details = self.apple_pay_recurring_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if redirect_response is not UNSET:
            field_dict["redirect_response"] = redirect_response
        if search_tags is not UNSET:
            field_dict["search_tags"] = search_tags
        if apple_pay_recurring_details is not UNSET:
            field_dict["apple_pay_recurring_details"] = apple_pay_recurring_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.apple_pay_recurring_details import ApplePayRecurringDetails
        from ..models.redirect_response import RedirectResponse

        d = dict(src_dict)

        def _parse_redirect_response(data: object) -> Union["RedirectResponse", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                redirect_response_type_1 = RedirectResponse.from_dict(data)

                return redirect_response_type_1
            except:  # noqa: E722
                pass
            return cast(Union["RedirectResponse", None, Unset], data)

        redirect_response = _parse_redirect_response(d.pop("redirect_response", UNSET))

        def _parse_search_tags(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                search_tags_type_0 = cast(list[str], data)

                return search_tags_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        search_tags = _parse_search_tags(d.pop("search_tags", UNSET))

        def _parse_apple_pay_recurring_details(data: object) -> Union["ApplePayRecurringDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                apple_pay_recurring_details_type_1 = ApplePayRecurringDetails.from_dict(data)

                return apple_pay_recurring_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ApplePayRecurringDetails", None, Unset], data)

        apple_pay_recurring_details = _parse_apple_pay_recurring_details(d.pop("apple_pay_recurring_details", UNSET))

        feature_metadata = cls(
            redirect_response=redirect_response,
            search_tags=search_tags,
            apple_pay_recurring_details=apple_pay_recurring_details,
        )

        feature_metadata.additional_properties = d
        return feature_metadata

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
