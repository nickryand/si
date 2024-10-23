from collections.abc import Iterable
from itertools import islice
import os
import boto3
import json
from typing import (
    Literal,
    NewType,
    NotRequired,
    Optional,
    TypeVar,
    TypedDict,
    Union,
    cast,
)
from pip._vendor import requests
import urllib.parse
from si_logger import logger

ExternalSubscriptionId = NewType("ExternalSubscriptionId", str)


class LagoEvent(TypedDict):
    transaction_id: str
    external_subscription_id: ExternalSubscriptionId
    timestamp: float
    code: str
    properties: dict[str, int]


T = TypeVar("T")


class LagoErrorResponse(TypedDict):
    code: str
    error_details: dict[str, dict[str, list[str]]]


def batch(iterable: Iterable[T], n):
    "Batch data into lists of length n. The last batch may be shorter."
    # batched('ABCDEFG', 3) --> ABC DEF G
    it = iter(iterable)
    while True:
        batch = list(islice(it, n))
        if not batch:
            return
        yield batch


class LagoHTTPError(Exception):
    def __init__(self, *args, json, **kwargs):
        super().__init__(*args, **kwargs)
        self.json = json


class LagoApi:
    @classmethod
    def from_env(cls, session: boto3.Session):
        lago_api_url = os.environ["LAGO_API_URL"]
        lago_api_token = os.environ.get("LAGO_API_TOKEN")
        if lago_api_token is None:
            lago_api_token_arn = os.environ["LAGO_API_TOKEN_ARN"]

            secretsmanager = session.client(service_name="secretsmanager")
            secret = secretsmanager.get_secret_value(SecretId=lago_api_token_arn)
            lago_api_token = json.loads(secret["SecretString"])["LAGO_API_TOKEN"]

        return LagoApi(lago_api_url, lago_api_token)

    def __init__(self, lago_api_url: str, lago_api_token: str):
        self.lago_api_url = lago_api_url
        self.lago_api_token = lago_api_token

    def request(self, method: str, path: str, **kwargs):
        response = requests.request(
            method,
            urllib.parse.urljoin(self.lago_api_url, path),
            headers={"Authorization": f"Bearer {self.lago_api_token}"},
            **kwargs,
        )
        try:
            response.raise_for_status()
        except requests.HTTPError as e:
            raise LagoHTTPError(
                f"{e.response.status_code} for {method} {path}: {e.response.text}",
                json=e.response.json(),
            ) from e

        return response

    def get(self, path: str):
        return self.request("GET", path)

    def delete(self, path: str):
        return self.request("DELETE", path)

    def put(self, path: str, json):
        return self.request("PUT", path, json=json)

    def post(self, path: str, json):
        return self.request("POST", path, json=json)

    def upload_events(self, events: Iterable[LagoEvent]):
        """
        Uploads events to Lago, batching them in groups of 100 (Lago's maximum).

        If a batch fails due to an event already having been uploaded, we retry the ones that
        try to upload each event
        individually.

        Exceptions:
        - HTTPError: Any HTTP error response except 422 (which indicates the event already exists)
        will be thrown.
        """

        total_events = 0
        new_events = 0
        for event_batch in batch(events, 100):
            total_events += len(event_batch)

            try:
                self.post("/api/v1/events/batch", {"events": event_batch})
                new_events += len(event_batch)

            except LagoHTTPError as e:

                # If the batch failed because some events were already uploaded, retry the rest
                if [e.json.get("status"), e.json.get("code")] != [
                    422,
                    "validation_errors",
                ]:
                    raise

                for error in e.json["error_details"].values():
                    if error.get("transaction_id") != ["value_already_exist"]:
                        raise

                # Retry the events that did not fail
                logger.debug(
                    f"Events already existed: {[event_batch[int(i)]['transaction_id'] for i in e.json["error_details"].keys()]}"
                )
                retry_event_batch = [
                    event
                    for (i, event) in enumerate(event_batch)
                    if str(i) not in e.json["error_details"].keys()
                ]

                if len(retry_event_batch) > 0:
                    logger.warning(
                        f"{len(e.json["error_details"])} / {len(event_batch)} events already existed. Reuploading remaining {len(retry_event_batch)} events."
                    )

                    # Retry the events that didn't fail. Any failure here is a real error
                    self.post("/api/v1/events/batch", {"events": retry_event_batch})
                    new_events += len(retry_event_batch)

        return new_events, total_events


class LagoResponseMetadata(TypedDict):
    current_page: int
    next_page: NotRequired[Optional[int]]
    prev_page: NotRequired[Optional[int]]
    total_pages: int
    total_count: int


LagoCurrency = Literal[
    "AED",
    "AFN",
    "ALL",
    "AMD",
    "ANG",
    "AOA",
    "ARS",
    "AUD",
    "AWG",
    "AZN",
    "BAM",
    "BBD",
    "BDT",
    "BGN",
    "BIF",
    "BMD",
    "BND",
    "BOB",
    "BRL",
    "BSD",
    "BWP",
    "BYN",
    "BZD",
    "CAD",
    "CDF",
    "CHF",
    "CLF",
    "CLP",
    "CNY",
    "COP",
    "CRC",
    "CVE",
    "CZK",
    "DJF",
    "DKK",
    "DOP",
    "DZD",
    "EGP",
    "ETB",
    "EUR",
    "FJD",
    "FKP",
    "GBP",
    "GEL",
    "GIP",
    "GMD",
    "GNF",
    "GTQ",
    "GYD",
    "HKD",
    "HNL",
    "HRK",
    "HTG",
    "HUF",
    "IDR",
    "ILS",
    "INR",
    "ISK",
    "JMD",
    "JPY",
    "KES",
    "KGS",
    "KHR",
    "KMF",
    "KRW",
    "KYD",
    "KZT",
    "LAK",
    "LBP",
    "LKR",
    "LRD",
    "LSL",
    "MAD",
    "MDL",
    "MGA",
    "MKD",
    "MMK",
    "MNT",
    "MOP",
    "MRO",
    "MUR",
    "MVR",
    "MWK",
    "MXN",
    "MYR",
    "MZN",
    "NAD",
    "NGN",
    "NIO",
    "NOK",
    "NPR",
    "NZD",
    "PAB",
    "PEN",
    "PGK",
    "PHP",
    "PKR",
    "PLN",
    "PYG",
    "QAR",
    "RON",
    "RSD",
    "RUB",
    "RWF",
    "SAR",
    "SBD",
    "SCR",
    "SEK",
    "SGD",
    "SHP",
    "SLL",
    "SOS",
    "SRD",
    "STD",
    "SZL",
    "THB",
    "TJS",
    "TOP",
    "TRY",
    "TTD",
    "TWD",
    "TZS",
    "UAH",
    "UGX",
    "USD",
    "UYU",
    "UZS",
    "VND",
    "VUV",
]


class LagoInvoice(TypedDict):
    lago_id: str
    sequential_id: NotRequired[int]
    number: str
    issuing_date: str
    payment_dispute_lost_at: NotRequired[str]
    payment_due_date: NotRequired[str]
    payment_overdue: NotRequired[bool]
    net_payment_term: NotRequired[int]
    invoice_type: Literal[
        "subscription", "add_on", "credit", "one_off", "progressive_billing"
    ]
    status: Literal["draft", "finalized", "voided", "failed"]
    payment_status: Literal["pending", "succeeded", "failed"]
    currency: LagoCurrency
    fees_amount_cents: int
    coupons_amount_cents: int
    credit_notes_amount_cents: int
    sub_total_excluding_taxes_amount_cents: int
    taxes_amount_cents: int
    sub_total_including_taxes_amount_cents: int
    prepaid_credit_amount_cents: int
    progressive_billing_credit_amount_cents: int
    total_amount_cents: int
    customer: dict[str, object]
    metadata: list[dict[str, object]]
    applied_taxes: list[dict[str, object]]
    applied_usage_thresholds: NotRequired[Optional[list[dict[str, object]]]]


class LagoInvoicesResponse(TypedDict):
    invoices: list[LagoInvoice]
    meta: LagoResponseMetadata


class LagoChargeProperties(TypedDict):
    graduated_ranges: list[dict[str, object]]
    graduated_percentage_ranges: list[dict[str, object]]
    amount: NotRequired[Optional[str]]
    free_units: NotRequired[Optional[int]]
    package_size: NotRequired[Optional[int]]
    rate: NotRequired[Optional[str]]
    fixed_amount: NotRequired[Optional[str]]
    free_units_per_events: NotRequired[Optional[int]]
    free_units_per_total_aggregation: NotRequired[Optional[str]]
    per_transaction_max_amount: NotRequired[Optional[str]]
    per_transaction_min_amount: NotRequired[Optional[str]]
    grouped_by: list[str]
    volume_ranges: list[dict[str, object]]


class LagoCharge(TypedDict):
    lago_id: str
    lago_billable_metric_id: str
    billable_metric_code: str
    invoice_display_name: NotRequired[Optional[str]]
    created_at: str
    charge_model: Literal[
        "standard",
        "graduated",
        "graduated_percentage",
        "package",
        "percentage",
        "volume",
        "dynamic",
    ]
    pay_in_advance: NotRequired[Optional[bool]]
    invoiceable: NotRequired[Optional[bool]]
    regroup_paid_fees: NotRequired[Optional[Literal["invoice"]]]
    prorated: NotRequired[Optional[bool]]
    min_amount_cents: NotRequired[Optional[int]]
    properties: LagoChargeProperties
    filters: list[dict[str, object]]
    taxes: list[dict[str, object]]


class LagoPlan(TypedDict):
    lago_id: str
    name: str
    invoice_display_name: NotRequired[Optional[str]]
    created_at: str
    code: str
    interval: Literal["weekly", "monthly", "quarterly", "yearly"]
    description: NotRequired[Optional[str]]
    amount_cents: int
    amount_currency: LagoCurrency
    trial_period: NotRequired[Optional[float]]
    pay_in_advance: bool
    bill_charges_monthly: NotRequired[Optional[bool]]
    active_subscriptions_count: int
    draft_invoices_count: int
    minimum_commitment: NotRequired[Optional[dict[str, object]]]
    charges: list[LagoCharge]
    taxes: list[dict[str, object]]
    usage_thresholds: list[dict[str, object]]


class LagoSubscription(TypedDict):
    lago_id: str
    external_id: str
    lago_customer_id: str
    external_customer_id: str
    billing_time: Literal["calendar", "anniversary"]
    name: NotRequired[Optional[str]]
    plan_code: str
    status: Literal["active", "pending", "terminated", "canceled"]
    created_at: str
    canceled_at: NotRequired[Optional[str]]
    started_at: NotRequired[Optional[str]]
    ending_at: str
    subscription_at: str
    terminated_at: NotRequired[Optional[str]]
    previous_plan_code: NotRequired[Optional[str]]
    next_plan_code: NotRequired[Optional[str]]
    downgrade_plan_date: NotRequired[Optional[str]]
    trial_ended_at: NotRequired[Optional[str]]
    plan: LagoPlan
