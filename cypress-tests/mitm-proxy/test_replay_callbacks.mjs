import assert from "node:assert/strict";
import test from "node:test";

import { mockReplay3ds } from "../cypress/support/mitmProxy.js";

const responseUrl =
  "http://localhost:8080/payments/pay_1/merchant_1/redirect/token/d9dc53cc-d1a1-4f57-9077-d20c2d614daa/response/braintree";
const completeUrl = responseUrl
  .replace("614daa", "614dab")
  .replace("/response/", "/complete/");

// Exercise the replay chain without a running browser, router or proxy.
function replay(attributes, saved, adminCallbacks = []) {
  const requests = [];
  const state = { paymentID: "pay_1", merchantId: "merchant_1" };
  globalThis.Cypress = {
    env: (key) =>
      ({
        currentTestIdHash: "test",
        CAPTURE_DIR: "/captures",
        PROXY_ADMIN_URL: "http://localhost:8001",
      })[key],
  };
  globalThis.DOMParser = class {
    parseFromString() {
      return { body: { getAttribute: (key) => attributes[key] || null } };
    }
  };
  globalThis.expect = (value, message) => ({
    to: {
      have: {
        length: (expected) => assert.equal(value.length, expected, message),
      },
      be: {
        within: (min, max) => assert.ok(value >= min && value <= max, message),
        a: (type) => {
          assert.equal(typeof value, type, message);
          return {
            and: {
              not: { be: { empty: assert.notEqual(value, "", message) } },
            },
          };
        },
      },
    },
  });
  globalThis.cy = {
    request: (options) => {
      requests.push(options);
      if (new URL(options.url).pathname === "/callbacks") {
        assert.deepEqual(options.qs, {
          payment_id: state.paymentID,
          merchant_id: state.merchantId,
        });
        return Promise.resolve({ body: { urls: adminCallbacks } });
      }
      return Promise.resolve({
        status: 200,
        headers: {},
        body: "redirect page",
      });
    },
    task: (name) => {
      assert.equal(name, "readFileOrNull");
      return Promise.resolve(saved);
    },
    wrap: (value) => Promise.resolve(value),
  };
  return mockReplay3ds(
    { get: (key) => state[key] },
    "braintree",
    "http://localhost:8080/start"
  ).then(() => requests);
}

test("replay still accepts a page with a single callback", () => {
  return replay(
    { "data-callback-url": completeUrl },
    { __redirect_segment: "redirect/complete/braintree", __body: {} }
  ).then((requests) => {
    assert.equal(requests.length, 2);
    assert.equal(requests[1].url, completeUrl);
  });
});

test("replay falls back to the proxy for callbacks absent from the page", () => {
  return replay(
    {},
    { __redirect_segment: "redirect/complete/braintree", __body: {} },
    [responseUrl, completeUrl]
  ).then((requests) => {
    assert.equal(requests.length, 3);
    assert.equal(requests[2].url, completeUrl);
  });
});

test("replay rejects a page callback for another payment", () => {
  return assert.rejects(
    replay(
      { "data-complete-callback-url": completeUrl.replace("pay_1", "pay_2") },
      { __redirect_segment: "redirect/complete/braintree", __body: {} }
    ),
    /one fresh router-issued callback for this payment/
  );
});

for (const [route, url] of [
  ["complete", completeUrl],
  ["response", responseUrl],
]) {
  test(`Braintree replays the current ${route} callback from the page`, () => {
    const saved = {
      __redirect_segment: `redirect/${route}/braintree`,
      __redirect_method: "POST",
      __body: { authentication_response: "recorded connector payload" },
    };
    return replay(
      {
        "data-callback-url": responseUrl,
        "data-complete-callback-url": completeUrl,
      },
      saved
    ).then((requests) => {
      assert.equal(requests.length, 2);
      assert.equal(requests[1].url, url);
      assert.equal(requests[1].method, "POST");
      assert.deepEqual(requests[1].body, saved.__body);
      assert.equal(requests[1].retryOnNetworkFailure, false);
    });
  });
}
