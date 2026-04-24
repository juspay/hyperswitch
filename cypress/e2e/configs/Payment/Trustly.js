import { standardBillingAddress } from "./Commons";
import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  connector_account_details: {
    auth_type: "SignatureKey",
    api_key: "gm_gaming_pnp",
    key1: "77da6b37-afc3-4f17-bf70-96073c4cbd5f",
    api_secret:
      "LS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tCk1JSUV2UUlCQURBTkJna3Foa2lHOXcwQkFRRUZBQVNDQktjd2dnU2pBZ0VBQW9JQkFRQ2Q4dkF0Y01ZSDhBdnUKeC96TjljVjRlVVFJbjJ5QTNiTERGRlNBVUgvMlNDU25OcHloY0NaOEhxOE5UN1RRNk1Ja1RQQ3dKbHNmUWl6SQpHNldNb21YN3hXeS8rUnNEVmVzM1RlSUMxaTJNQ0NtU2hMY0RIUWUvMW5kbVRid2hsbU1kQkdIR1JJNDBjWjVjCkRpWWtWNXBPTWt2NlprVDF1ZHlXM1VQWjBYK0xEbnVzdlZhTm1rSHkzeDdtRC9kdnZRK2xtcVNURmwra3N0dXYKdWIvaFRwcVpwb2RhY2VXcSs2cmF5bnJTdEVSdmNjZDVBbUdscGhxVEVzc3dFYlNINytlUTZDVEdXRlRKYWtBbQpyRHE2dUY0NHpMWEN0d3QwVnRTdlFYYkFQL1NaUzV5NnlZQXo4a3NUNlBNcEdHT09FTkkvNHkrbnpPdjlkY3JhCjY1OFF1Vzh2QWdNQkFBRUNnZ0VBQ25uTkZEWjJscHJhaFJaejNmVXBzN2tzbWZzamtkTWliVzNid2p3L1FIRVQKSis2blZNM0F2N0FKVEROaWVyaWYvUk1IK1B3ODd2QjUvVFpZTkl1bUxVZGExSFlMejBkK041ek5jSzFqNHFCbgo5c3AybU81RCtxVkVKZ28zS3NNZXI3My8vaUVZald5UkR1bjJxRkpuR1pOSEZzUjhXUS9nRERiV0ErR3hPVGU4Cm5XelhqbFRZWE5QY3RhRVM2UGFseWo4cGViaTRxUXdtZVVIekM5NjdHdjNiMEZ1WExWdzdyUDdjWW5pN3dzbGEKZzJ1alNDSk1xeHZ1WVB1Nk1jejZPUVRTMjBiam55eXJJYVhIUlc4a0NhTjR4dVczL2JSNStUUGxYam9FOWNxTwpRbk8ySU5NRC9RVFZoUVdsbCt0a2wxU2tkcXFZZGExN0RqRVJsU3FKQVFLQmdRQzY2ZXpMMzRjYmlEY3NqOWU0Cm5zbGNYY2U3OHhzK3JKc3I3UVhyM0VsTEpMYlU4RTRFU01YQithdUNmNC9VVnJEWmRBNkgvWWEzR0lZUzMvL2IKNERtR1IzZ3VpbUxaZSs4V1NlRVFmeXN3NDY2YTd2T2tZUkpsTGhyalpTd0Q5amRqRmRMSHRMdCtYRjVWaWluQgpRY05GRVROTTZMNnp6MVdhUlJqandrd3Z1UUtCZ1FEWVZGR3BidFloNjZubDEzTWIvSzI2STJqVmtiZDBheFoxClZPVmNlTktrVW53RWdLSldWWElBTlU5ck5mRlhpN3NlNVg1SzFCVmVTcHpQSEVaRnpBcDdVYWtQQ3dNTm9wYUQKSzBTRHVORm96MzRVa0FFSW8vZFhJL3hCUDNqbFZ4ZkJoQzRnZzdPSVBTV2pBaWxnS2dHK0JlL2wyWDhKWEpRTgpxT1huMjJCNkp3S0JnSGhkU1hMa0orSVA3cy9RZFc5Yk9YbzBuZm1uak1Ub2JDaDJReGpteTRBTFRYMkVuZ2plClFCTXd6MFNERnNEN2Jua3A1bTJtVW9rM3pxYjYvbzIrTCswTHV4WGxZZENCb2E0dHR6UmpZQkhrbkwwQzRYemEKVWZrOEhtNk82VEJIN3RUczZjWlcyV0orNHZTY3UxVExINDhyaTJpY2ZLblgrMTBUMy9wVFZiSnBBb0dBS0hjbApTMTlMT004Zldib0NjekxCd0hiTkxsM3loaGxkK3hhbFRMWGhHUkhBMXdyRTB3ZHFxclZPSk16VFZ6L3hBYWVHCkJ6TkN4LytHODRyamJqenJuUU82YnZEdFlraU9oUHk3OVRQR0lDZm4rcXF4TnMrTS9jVGljOFlPdEQrbFZ2S0YKdWxsSVpPeWpOS203Mlp3aDlVeWhBa3E3NDZGb3BHRjZsL05HU2JNQ2dZRUFuQllpS3RBVEtsWmtCdDgxOGpxegozY3NxK2RPK3VjeHFyQVNNbmdDdytNRjN3VmhjdkhPelo4aEY3aHg2cjB0RG5hdGFDMWhsck9ucjF5Q1NyQ3lVCkhzZTBtRm9Fdm1Tc2ozK0RHRzFycFVhQWtuSUJlQ2dQWERMTE04ckFkd0hhdTl4NHVvYnFYVG5wYldwODloZUkKQXU0VURvN2k2Q3VjNEZVdjdyVXB6Qnc9PQotLS0tLUVORCBQUklWQVRFIEtFWS0tLS0tCg==",
  },
  bank_redirect_pm: {
    PaymentIntent: (_paymentMethodType) =>
      getCustomExchange({
        Request: {
          currency: "EUR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Trustly: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "trustly",
        payment_method_data: {
          bank_redirect: {
            trustly: {
              country: "NL",
            },
          },
        },
        billing: standardBillingAddress,
      },
    }),
    Refund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },
};
