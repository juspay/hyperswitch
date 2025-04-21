from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.blocklist_data_kind import BlocklistDataKind
from ...models.blocklist_response import BlocklistResponse
from ...types import UNSET, Response


def _get_kwargs(
    *,
    data_kind: BlocklistDataKind,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_data_kind = data_kind.value
    params["data_kind"] = json_data_kind

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/blocklist",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, BlocklistResponse]]:
    if response.status_code == 200:
        response_200 = BlocklistResponse.from_dict(response.json())

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
) -> Response[Union[Any, BlocklistResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    data_kind: BlocklistDataKind,
) -> Response[Union[Any, BlocklistResponse]]:
    """
    Args:
        data_kind (BlocklistDataKind):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, BlocklistResponse]]
    """

    kwargs = _get_kwargs(
        data_kind=data_kind,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    data_kind: BlocklistDataKind,
) -> Optional[Union[Any, BlocklistResponse]]:
    """
    Args:
        data_kind (BlocklistDataKind):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, BlocklistResponse]
    """

    return sync_detailed(
        client=client,
        data_kind=data_kind,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    data_kind: BlocklistDataKind,
) -> Response[Union[Any, BlocklistResponse]]:
    """
    Args:
        data_kind (BlocklistDataKind):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, BlocklistResponse]]
    """

    kwargs = _get_kwargs(
        data_kind=data_kind,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    data_kind: BlocklistDataKind,
) -> Optional[Union[Any, BlocklistResponse]]:
    """
    Args:
        data_kind (BlocklistDataKind):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, BlocklistResponse]
    """

    return (
        await asyncio_detailed(
            client=client,
            data_kind=data_kind,
        )
    ).parsed
