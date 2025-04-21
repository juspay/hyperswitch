import datetime
from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.dispute_response import DisputeResponse
from ...models.dispute_stage import DisputeStage
from ...models.dispute_status import DisputeStatus
from ...types import UNSET, Response, Unset


def _get_kwargs(
    *,
    limit: Union[None, Unset, int] = UNSET,
    dispute_status: Union[DisputeStatus, None, Unset] = UNSET,
    dispute_stage: Union[DisputeStage, None, Unset] = UNSET,
    reason: Union[None, Unset, str] = UNSET,
    connector: Union[None, Unset, str] = UNSET,
    received_time: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lte: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gte: Union[None, Unset, datetime.datetime] = UNSET,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_limit: Union[None, Unset, int]
    if isinstance(limit, Unset):
        json_limit = UNSET
    else:
        json_limit = limit
    params["limit"] = json_limit

    json_dispute_status: Union[None, Unset, str]
    if isinstance(dispute_status, Unset):
        json_dispute_status = UNSET
    elif isinstance(dispute_status, DisputeStatus):
        json_dispute_status = dispute_status.value
    else:
        json_dispute_status = dispute_status
    params["dispute_status"] = json_dispute_status

    json_dispute_stage: Union[None, Unset, str]
    if isinstance(dispute_stage, Unset):
        json_dispute_stage = UNSET
    elif isinstance(dispute_stage, DisputeStage):
        json_dispute_stage = dispute_stage.value
    else:
        json_dispute_stage = dispute_stage
    params["dispute_stage"] = json_dispute_stage

    json_reason: Union[None, Unset, str]
    if isinstance(reason, Unset):
        json_reason = UNSET
    else:
        json_reason = reason
    params["reason"] = json_reason

    json_connector: Union[None, Unset, str]
    if isinstance(connector, Unset):
        json_connector = UNSET
    else:
        json_connector = connector
    params["connector"] = json_connector

    json_received_time: Union[None, Unset, str]
    if isinstance(received_time, Unset):
        json_received_time = UNSET
    elif isinstance(received_time, datetime.datetime):
        json_received_time = received_time.isoformat()
    else:
        json_received_time = received_time
    params["received_time"] = json_received_time

    json_received_time_lt: Union[None, Unset, str]
    if isinstance(received_time_lt, Unset):
        json_received_time_lt = UNSET
    elif isinstance(received_time_lt, datetime.datetime):
        json_received_time_lt = received_time_lt.isoformat()
    else:
        json_received_time_lt = received_time_lt
    params["received_time.lt"] = json_received_time_lt

    json_received_time_gt: Union[None, Unset, str]
    if isinstance(received_time_gt, Unset):
        json_received_time_gt = UNSET
    elif isinstance(received_time_gt, datetime.datetime):
        json_received_time_gt = received_time_gt.isoformat()
    else:
        json_received_time_gt = received_time_gt
    params["received_time.gt"] = json_received_time_gt

    json_received_time_lte: Union[None, Unset, str]
    if isinstance(received_time_lte, Unset):
        json_received_time_lte = UNSET
    elif isinstance(received_time_lte, datetime.datetime):
        json_received_time_lte = received_time_lte.isoformat()
    else:
        json_received_time_lte = received_time_lte
    params["received_time.lte"] = json_received_time_lte

    json_received_time_gte: Union[None, Unset, str]
    if isinstance(received_time_gte, Unset):
        json_received_time_gte = UNSET
    elif isinstance(received_time_gte, datetime.datetime):
        json_received_time_gte = received_time_gte.isoformat()
    else:
        json_received_time_gte = received_time_gte
    params["received_time.gte"] = json_received_time_gte

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/disputes/list",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, list["DisputeResponse"]]]:
    if response.status_code == 200:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in _response_200:
            response_200_item = DisputeResponse.from_dict(response_200_item_data)

            response_200.append(response_200_item)

        return response_200
    if response.status_code == 401:
        response_401 = cast(Any, None)
        return response_401
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, list["DisputeResponse"]]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    dispute_status: Union[DisputeStatus, None, Unset] = UNSET,
    dispute_stage: Union[DisputeStage, None, Unset] = UNSET,
    reason: Union[None, Unset, str] = UNSET,
    connector: Union[None, Unset, str] = UNSET,
    received_time: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lte: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gte: Union[None, Unset, datetime.datetime] = UNSET,
) -> Response[Union[Any, list["DisputeResponse"]]]:
    """Disputes - List Disputes

     Lists all the Disputes for a merchant

    Args:
        limit (Union[None, Unset, int]):
        dispute_status (Union[DisputeStatus, None, Unset]):
        dispute_stage (Union[DisputeStage, None, Unset]):
        reason (Union[None, Unset, str]):
        connector (Union[None, Unset, str]):
        received_time (Union[None, Unset, datetime.datetime]):
        received_time_lt (Union[None, Unset, datetime.datetime]):
        received_time_gt (Union[None, Unset, datetime.datetime]):
        received_time_lte (Union[None, Unset, datetime.datetime]):
        received_time_gte (Union[None, Unset, datetime.datetime]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, list['DisputeResponse']]]
    """

    kwargs = _get_kwargs(
        limit=limit,
        dispute_status=dispute_status,
        dispute_stage=dispute_stage,
        reason=reason,
        connector=connector,
        received_time=received_time,
        received_time_lt=received_time_lt,
        received_time_gt=received_time_gt,
        received_time_lte=received_time_lte,
        received_time_gte=received_time_gte,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    dispute_status: Union[DisputeStatus, None, Unset] = UNSET,
    dispute_stage: Union[DisputeStage, None, Unset] = UNSET,
    reason: Union[None, Unset, str] = UNSET,
    connector: Union[None, Unset, str] = UNSET,
    received_time: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lte: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gte: Union[None, Unset, datetime.datetime] = UNSET,
) -> Optional[Union[Any, list["DisputeResponse"]]]:
    """Disputes - List Disputes

     Lists all the Disputes for a merchant

    Args:
        limit (Union[None, Unset, int]):
        dispute_status (Union[DisputeStatus, None, Unset]):
        dispute_stage (Union[DisputeStage, None, Unset]):
        reason (Union[None, Unset, str]):
        connector (Union[None, Unset, str]):
        received_time (Union[None, Unset, datetime.datetime]):
        received_time_lt (Union[None, Unset, datetime.datetime]):
        received_time_gt (Union[None, Unset, datetime.datetime]):
        received_time_lte (Union[None, Unset, datetime.datetime]):
        received_time_gte (Union[None, Unset, datetime.datetime]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, list['DisputeResponse']]
    """

    return sync_detailed(
        client=client,
        limit=limit,
        dispute_status=dispute_status,
        dispute_stage=dispute_stage,
        reason=reason,
        connector=connector,
        received_time=received_time,
        received_time_lt=received_time_lt,
        received_time_gt=received_time_gt,
        received_time_lte=received_time_lte,
        received_time_gte=received_time_gte,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    dispute_status: Union[DisputeStatus, None, Unset] = UNSET,
    dispute_stage: Union[DisputeStage, None, Unset] = UNSET,
    reason: Union[None, Unset, str] = UNSET,
    connector: Union[None, Unset, str] = UNSET,
    received_time: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lte: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gte: Union[None, Unset, datetime.datetime] = UNSET,
) -> Response[Union[Any, list["DisputeResponse"]]]:
    """Disputes - List Disputes

     Lists all the Disputes for a merchant

    Args:
        limit (Union[None, Unset, int]):
        dispute_status (Union[DisputeStatus, None, Unset]):
        dispute_stage (Union[DisputeStage, None, Unset]):
        reason (Union[None, Unset, str]):
        connector (Union[None, Unset, str]):
        received_time (Union[None, Unset, datetime.datetime]):
        received_time_lt (Union[None, Unset, datetime.datetime]):
        received_time_gt (Union[None, Unset, datetime.datetime]):
        received_time_lte (Union[None, Unset, datetime.datetime]):
        received_time_gte (Union[None, Unset, datetime.datetime]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, list['DisputeResponse']]]
    """

    kwargs = _get_kwargs(
        limit=limit,
        dispute_status=dispute_status,
        dispute_stage=dispute_stage,
        reason=reason,
        connector=connector,
        received_time=received_time,
        received_time_lt=received_time_lt,
        received_time_gt=received_time_gt,
        received_time_lte=received_time_lte,
        received_time_gte=received_time_gte,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    dispute_status: Union[DisputeStatus, None, Unset] = UNSET,
    dispute_stage: Union[DisputeStage, None, Unset] = UNSET,
    reason: Union[None, Unset, str] = UNSET,
    connector: Union[None, Unset, str] = UNSET,
    received_time: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gt: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_lte: Union[None, Unset, datetime.datetime] = UNSET,
    received_time_gte: Union[None, Unset, datetime.datetime] = UNSET,
) -> Optional[Union[Any, list["DisputeResponse"]]]:
    """Disputes - List Disputes

     Lists all the Disputes for a merchant

    Args:
        limit (Union[None, Unset, int]):
        dispute_status (Union[DisputeStatus, None, Unset]):
        dispute_stage (Union[DisputeStage, None, Unset]):
        reason (Union[None, Unset, str]):
        connector (Union[None, Unset, str]):
        received_time (Union[None, Unset, datetime.datetime]):
        received_time_lt (Union[None, Unset, datetime.datetime]):
        received_time_gt (Union[None, Unset, datetime.datetime]):
        received_time_lte (Union[None, Unset, datetime.datetime]):
        received_time_gte (Union[None, Unset, datetime.datetime]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, list['DisputeResponse']]
    """

    return (
        await asyncio_detailed(
            client=client,
            limit=limit,
            dispute_status=dispute_status,
            dispute_stage=dispute_stage,
            reason=reason,
            connector=connector,
            received_time=received_time,
            received_time_lt=received_time_lt,
            received_time_gt=received_time_gt,
            received_time_lte=received_time_lte,
            received_time_gte=received_time_gte,
        )
    ).parsed
