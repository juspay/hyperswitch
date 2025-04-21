from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.dispute_response import DisputeResponse
from ...types import Response


def _get_kwargs(
    dispute_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/disputes/{dispute_id}",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, DisputeResponse]]:
    if response.status_code == 200:
        response_200 = DisputeResponse.from_dict(response.json())

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
) -> Response[Union[Any, DisputeResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    dispute_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, DisputeResponse]]:
    """Disputes - Retrieve Dispute

     Retrieves a dispute

    Args:
        dispute_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, DisputeResponse]]
    """

    kwargs = _get_kwargs(
        dispute_id=dispute_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    dispute_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, DisputeResponse]]:
    """Disputes - Retrieve Dispute

     Retrieves a dispute

    Args:
        dispute_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, DisputeResponse]
    """

    return sync_detailed(
        dispute_id=dispute_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    dispute_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, DisputeResponse]]:
    """Disputes - Retrieve Dispute

     Retrieves a dispute

    Args:
        dispute_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, DisputeResponse]]
    """

    kwargs = _get_kwargs(
        dispute_id=dispute_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    dispute_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, DisputeResponse]]:
    """Disputes - Retrieve Dispute

     Retrieves a dispute

    Args:
        dispute_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, DisputeResponse]
    """

    return (
        await asyncio_detailed(
            dispute_id=dispute_id,
            client=client,
        )
    ).parsed
