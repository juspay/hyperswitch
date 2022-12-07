import { group } from 'k6';
import { Counter } from "k6/metrics";
import { readBaseline, storeResult } from "./helper/compare-result.js";
import { setup_merchant_apikey } from "./helper/setup.js";
import paymentCreateAndConfirmFunc from './payment-create-and-confirm.js';
import paymentConfirmFunc from './payment-confirm.js';

export const requests = new Counter("http_reqs");

const baseline = readBaseline("rps");

export const options = {
    scenarios: {
        contacts: {
            executor: 'per-vu-iterations',
            vus: 10,
            iterations: 100,
            maxDuration: '5m',
        },
    },
};

export function setup() {
    return setup_merchant_apikey()
}

export default function (data) {
    group("create payment and confirm", function() { paymentCreateAndConfirmFunc(data) });
    group("create confirmed payment", function() { paymentConfirmFunc(data) });
}

export function handleSummary(data) {
    return storeResult("rps", baseline, data)
}
