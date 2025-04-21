from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payment_method_create import PaymentMethodCreate
from ...models.payment_method_response import PaymentMethodResponse
from ...types import Response


def _get_kwargs(
    *,
    body: PaymentMethodCreate,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/payment_methods",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PaymentMethodResponse]]:
    if response.status_code == 200:
        response_200 = PaymentMethodResponse.from_dict(response.json())

        return response_200
    if response.status_code == 400:
        response_400 = cast(Any, None)
        return response_400
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, PaymentMethodResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    body: PaymentMethodCreate,
) -> Response[Union[Any, PaymentMethodResponse]]:
    """PaymentMethods - Create

     Creates and stores a payment method against a customer.
    In case of cards, this API should be used only by PCI compliant merchants.

    Args:
        body (PaymentMethodCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PaymentMethodResponse]]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    body: PaymentMethodCreate,
) -> Optional[Union[Any, PaymentMethodResponse]]:
    """PaymentMethods - Create

     Creates and stores a payment method against a customer.
    In case of cards, this API should be used only by PCI compliant merchants.

    Args:
        body (PaymentMethodCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PaymentMethodResponse]
    """

    return sync_detailed(
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: PaymentMethodCreate,
) -> Response[Union[Any, PaymentMethodResponse]]:
    """PaymentMethods - Create

     Creates and stores a payment method against a customer.
    In case of cards, this API should be used only by PCI compliant merchants.

    Args:
        body (PaymentMethodCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PaymentMethodResponse]]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    body: PaymentMethodCreate,
) -> Optional[Union[Any, PaymentMethodResponse]]:
    """PaymentMethods - Create

     Creates and stores a payment method against a customer.
    In case of cards, this API should be used only by PCI compliant merchants.

    Args:
        body (PaymentMethodCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PaymentMethodResponse]
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
        )
    ).parsed
