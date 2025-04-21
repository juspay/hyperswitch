from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payout_create_response import PayoutCreateResponse
from ...types import UNSET, Response, Unset


def _get_kwargs(
    payout_id: str,
    *,
    force_sync: Union[None, Unset, bool] = UNSET,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_force_sync: Union[None, Unset, bool]
    if isinstance(force_sync, Unset):
        json_force_sync = UNSET
    else:
        json_force_sync = force_sync
    params["force_sync"] = json_force_sync

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/payouts/{payout_id}",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PayoutCreateResponse]]:
    if response.status_code == 200:
        response_200 = PayoutCreateResponse.from_dict(response.json())

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
) -> Response[Union[Any, PayoutCreateResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    force_sync: Union[None, Unset, bool] = UNSET,
) -> Response[Union[Any, PayoutCreateResponse]]:
    """Payouts - Retrieve

    Args:
        payout_id (str):
        force_sync (Union[None, Unset, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PayoutCreateResponse]]
    """

    kwargs = _get_kwargs(
        payout_id=payout_id,
        force_sync=force_sync,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    force_sync: Union[None, Unset, bool] = UNSET,
) -> Optional[Union[Any, PayoutCreateResponse]]:
    """Payouts - Retrieve

    Args:
        payout_id (str):
        force_sync (Union[None, Unset, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PayoutCreateResponse]
    """

    return sync_detailed(
        payout_id=payout_id,
        client=client,
        force_sync=force_sync,
    ).parsed


async def asyncio_detailed(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    force_sync: Union[None, Unset, bool] = UNSET,
) -> Response[Union[Any, PayoutCreateResponse]]:
    """Payouts - Retrieve

    Args:
        payout_id (str):
        force_sync (Union[None, Unset, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PayoutCreateResponse]]
    """

    kwargs = _get_kwargs(
        payout_id=payout_id,
        force_sync=force_sync,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    payout_id: str,
    *,
    client: AuthenticatedClient,
    force_sync: Union[None, Unset, bool] = UNSET,
) -> Optional[Union[Any, PayoutCreateResponse]]:
    """Payouts - Retrieve

    Args:
        payout_id (str):
        force_sync (Union[None, Unset, bool]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PayoutCreateResponse]
    """

    return (
        await asyncio_detailed(
            payout_id=payout_id,
            client=client,
            force_sync=force_sync,
        )
    ).parsed
