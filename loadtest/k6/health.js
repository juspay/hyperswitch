import http from "k6/http";
import { sleep, check } from "k6";
import { Counter } from "k6/metrics";
import { readBaseline, storeResult } from "./helper/compare-result.js";

export const requests = new Counter('http_reqs');

const baseline = readBaseline("health");

export const options = {
    stages: [
        { duration: "10s", target: 25 },        // ramp up users to 25 in 10 seconds
        { duration: "10s", target: 25 },        // maintain 25 users for 10 seconds
        { duration: "10s", target: 0 }          // ramp down to 0 users in 10 seconds
    ],
    thresholds: {
        "http_req_duration": ["p(90) < 15"],   // 90% of requests must finish within 15ms.
    },
};

export default function () {
    const res = http.get("http://router-server:8080/health");
    check(res, {
        "health status 200": (r) => r.status === 200,
    });
}

export function handleSummary(data) {
    return storeResult("health", baseline, data)
}
