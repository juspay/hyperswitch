import datetime
from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...types import UNSET, Response


def _get_kwargs(
    *,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: int,
    created: datetime.datetime,
    created_lt: datetime.datetime,
    created_gt: datetime.datetime,
    created_lte: datetime.datetime,
    created_gte: datetime.datetime,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    params["customer_id"] = customer_id

    params["starting_after"] = starting_after

    params["ending_before"] = ending_before

    params["limit"] = limit

    json_created = created.isoformat()
    params["created"] = json_created

    json_created_lt = created_lt.isoformat()
    params["created_lt"] = json_created_lt

    json_created_gt = created_gt.isoformat()
    params["created_gt"] = json_created_gt

    json_created_lte = created_lte.isoformat()
    params["created_lte"] = json_created_lte

    json_created_gte = created_gte.isoformat()
    params["created_gte"] = json_created_gte

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/payments/list",
        "params": params,
    }

    return _kwargs


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[Any]:
    if response.status_code == 404:
        return None
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[Any]:
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
    limit: int,
    created: datetime.datetime,
    created_lt: datetime.datetime,
    created_gt: datetime.datetime,
    created_lte: datetime.datetime,
    created_gte: datetime.datetime,
) -> Response[Any]:
    """Payments - List

     To list the *payments*

    Args:
        customer_id (str):
        starting_after (str):
        ending_before (str):
        limit (int):
        created (datetime.datetime):
        created_lt (datetime.datetime):
        created_gt (datetime.datetime):
        created_lte (datetime.datetime):
        created_gte (datetime.datetime):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        customer_id=customer_id,
        starting_after=starting_after,
        ending_before=ending_before,
        limit=limit,
        created=created,
        created_lt=created_lt,
        created_gt=created_gt,
        created_lte=created_lte,
        created_gte=created_gte,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    customer_id: str,
    starting_after: str,
    ending_before: str,
    limit: int,
    created: datetime.datetime,
    created_lt: datetime.datetime,
    created_gt: datetime.datetime,
    created_lte: datetime.datetime,
    created_gte: datetime.datetime,
) -> Response[Any]:
    """Payments - List

     To list the *payments*

    Args:
        customer_id (str):
        starting_after (str):
        ending_before (str):
        limit (int):
        created (datetime.datetime):
        created_lt (datetime.datetime):
        created_gt (datetime.datetime):
        created_lte (datetime.datetime):
        created_gte (datetime.datetime):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        customer_id=customer_id,
        starting_after=starting_after,
        ending_before=ending_before,
        limit=limit,
        created=created,
        created_lt=created_lt,
        created_gt=created_gt,
        created_lte=created_lte,
        created_gte=created_gte,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)
