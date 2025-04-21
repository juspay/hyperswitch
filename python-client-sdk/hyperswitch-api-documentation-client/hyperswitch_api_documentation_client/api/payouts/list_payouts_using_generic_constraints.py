from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payout_list_response import PayoutListResponse
from ...types import UNSET, Response


def _get_kwargs(
    *,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: str,
    created: str,
    time_range: str,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    params["customer_id"] = customer_id

    params["starting_after"] = starting_after

    params["ending_before"] = ending_before

    params["limit"] = limit

    params["created"] = created

    params["time_range"] = time_range

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/payouts/list",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PayoutListResponse]]:
    if response.status_code == 200:
        response_200 = PayoutListResponse.from_dict(response.json())

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
) -> Response[Union[Any, PayoutListResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: str,
    created: str,
    time_range: str,
) -> Response[Union[Any, PayoutListResponse]]:
    """Payouts - List

    Args:
        customer_id (str):
        starting_after (str):
        ending_before (str):
        limit (str):
        created (str):
        time_range (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PayoutListResponse]]
    """

    kwargs = _get_kwargs(
        customer_id=customer_id,
        starting_after=starting_after,
        ending_before=ending_before,
        limit=limit,
        created=created,
        time_range=time_range,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: str,
    created: str,
    time_range: str,
) -> Optional[Union[Any, PayoutListResponse]]:
    """Payouts - List

    Args:
        customer_id (str):
        starting_after (str):
        ending_before (str):
        limit (str):
        created (str):
        time_range (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PayoutListResponse]
    """

    return sync_detailed(
        client=client,
        customer_id=customer_id,
        starting_after=starting_after,
        ending_before=ending_before,
        limit=limit,
        created=created,
        time_range=time_range,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: str,
    created: str,
    time_range: str,
) -> Response[Union[Any, PayoutListResponse]]:
    """Payouts - List

    Args:
        customer_id (str):
        starting_after (str):
        ending_before (str):
        limit (str):
        created (str):
        time_range (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PayoutListResponse]]
    """

    kwargs = _get_kwargs(
        customer_id=customer_id,
        starting_after=starting_after,
        ending_before=ending_before,
        limit=limit,
        created=created,
        time_range=time_range,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: str,
    created: str,
    time_range: str,
) -> Optional[Union[Any, PayoutListResponse]]:
    """Payouts - List

    Args:
        customer_id (str):
        starting_after (str):
        ending_before (str):
        limit (str):
        created (str):
        time_range (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PayoutListResponse]
    """

    return (
        await asyncio_detailed(
            client=client,
            customer_id=customer_id,
            starting_after=starting_after,
            ending_before=ending_before,
            limit=limit,
            created=created,
            time_range=time_range,
        )
    ).parsed
