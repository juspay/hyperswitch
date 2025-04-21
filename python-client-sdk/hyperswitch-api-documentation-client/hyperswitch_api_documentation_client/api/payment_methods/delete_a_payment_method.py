from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payment_method_delete_response import PaymentMethodDeleteResponse
from ...types import Response


def _get_kwargs(
    method_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": f"/payment_methods/{method_id}",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PaymentMethodDeleteResponse]]:
    if response.status_code == 200:
        response_200 = PaymentMethodDeleteResponse.from_dict(response.json())

        return response_200
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, PaymentMethodDeleteResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    method_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, PaymentMethodDeleteResponse]]:
    """Payment Method - Delete

     Deletes a payment method of a customer.

    Args:
        method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PaymentMethodDeleteResponse]]
    """

    kwargs = _get_kwargs(
        method_id=method_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    method_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, PaymentMethodDeleteResponse]]:
    """Payment Method - Delete

     Deletes a payment method of a customer.

    Args:
        method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PaymentMethodDeleteResponse]
    """

    return sync_detailed(
        method_id=method_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    method_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, PaymentMethodDeleteResponse]]:
    """Payment Method - Delete

     Deletes a payment method of a customer.

    Args:
        method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PaymentMethodDeleteResponse]]
    """

    kwargs = _get_kwargs(
        method_id=method_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    method_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, PaymentMethodDeleteResponse]]:
    """Payment Method - Delete

     Deletes a payment method of a customer.

    Args:
        method_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PaymentMethodDeleteResponse]
    """

    return (
        await asyncio_detailed(
            method_id=method_id,
            client=client,
        )
    ).parsed
