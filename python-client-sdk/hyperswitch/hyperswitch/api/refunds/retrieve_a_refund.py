from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.refund_response import RefundResponse
from ...types import Response


def _get_kwargs(
    refund_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/refunds/{refund_id}",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RefundResponse]]:
    if response.status_code == 200:
        response_200 = RefundResponse.from_dict(response.json())

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
) -> Response[Union[Any, RefundResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    refund_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, RefundResponse]]:
    """Refunds - Retrieve

     Retrieves a Refund. This may be used to get the status of a previously initiated refund

    Args:
        refund_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RefundResponse]]
    """

    kwargs = _get_kwargs(
        refund_id=refund_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    refund_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, RefundResponse]]:
    """Refunds - Retrieve

     Retrieves a Refund. This may be used to get the status of a previously initiated refund

    Args:
        refund_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RefundResponse]
    """

    return sync_detailed(
        refund_id=refund_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    refund_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, RefundResponse]]:
    """Refunds - Retrieve

     Retrieves a Refund. This may be used to get the status of a previously initiated refund

    Args:
        refund_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RefundResponse]]
    """

    kwargs = _get_kwargs(
        refund_id=refund_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    refund_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, RefundResponse]]:
    """Refunds - Retrieve

     Retrieves a Refund. This may be used to get the status of a previously initiated refund

    Args:
        refund_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RefundResponse]
    """

    return (
        await asyncio_detailed(
            refund_id=refund_id,
            client=client,
        )
    ).parsed
