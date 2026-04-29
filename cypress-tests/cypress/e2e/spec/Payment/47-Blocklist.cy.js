import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Commons";

let globalState;
const generateUniqueBin = (digits) => {
  const base = Math.pow(10, digits - 1);
  const range = Math.pow(10, digits) - base;
  return String(base + Math.floor(Math.random() * range));
};

const TEST_CARD_BIN = generateUniqueBin(6);
const TEST_EXTENDED_BIN = generateUniqueBin(8);

const getHeaders = () => ({
  "api-key": globalState.get("apiKey"),
  "Content-Type": "application/json",
});

const getBaseUrl = () => globalState.get("baseUrl");

describe("Blocklist CRUD Operations", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card BIN Blocklist Operations", () => {
    it("should add card_bin to blocklist successfully", () => {
      const data = connectorDetails.Blocklist.CreateCardBin;

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { ...fixtures.blocklistCreateBody, type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(data.Response.status);
        expect(response.body).to.have.property("fingerprint_id", TEST_CARD_BIN);
        expect(response.body).to.have.property("data_kind", "card_bin");
      });

      cy.request({
        method: "GET",
        url: `${getBaseUrl()}/blocklist?data_kind=card_bin`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("count");
        expect(response.body).to.have.property("total_count");
        expect(response.body).to.have.property("data");
        expect(response.body.data).to.be.an("array");
      });

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
      });

      cy.request({
        method: "GET",
        url: `${getBaseUrl()}/blocklist?data_kind=card_bin`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("data");
        expect(response.body.data).to.be.an("array");
        const match = response.body.data.find(
          (entry) => entry.fingerprint_id === TEST_CARD_BIN
        );
        expect(match).to.be.undefined;
      });
    });

    it("should reject duplicate card_bin blocklist entry", () => {
      const duplicateData = connectorDetails.Blocklist.CreateDuplicate;

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { ...fixtures.blocklistCreateBody, type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("fingerprint_id", TEST_CARD_BIN);
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { ...fixtures.blocklistCreateBody, type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(duplicateData.Response.status);
        expect(response.body.error.message).to.include("already blocked");
      });

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
      });
    });
  });

  context("Extended Card BIN Blocklist Operations", () => {
    it("should add extended_card_bin to blocklist successfully", () => {
      const data = connectorDetails.Blocklist.CreateExtendedCardBin;

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
        failOnStatusCode: false,
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(data.Response.status);
        expect(response.body).to.have.property("fingerprint_id", TEST_EXTENDED_BIN);
        expect(response.body).to.have.property("data_kind", "extended_card_bin");
      });

      cy.request({
        method: "GET",
        url: `${getBaseUrl()}/blocklist?data_kind=extended_card_bin`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("count");
        expect(response.body).to.have.property("data");
        expect(response.body.data).to.be.an("array");
      });

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
      });
    });
  });

  context("Blocklist Guard Toggle Operations", () => {
    it("should disable and re-enable blocklist guard", () => {
      const toggleDisableData = connectorDetails.Blocklist.ToggleDisable;
      const toggleEnableData = connectorDetails.Blocklist.ToggleEnable;

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist/toggle?status=false`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(toggleDisableData.Response.status);
        expect(response.body.blocklist_guard_status).to.eq("disabled");
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist/toggle?status=true`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(toggleEnableData.Response.status);
        expect(response.body.blocklist_guard_status).to.eq("enabled");
      });
    });
  });

  context("Full Blocklist Lifecycle", () => {
    it("should perform complete blocklist lifecycle - add all types, list, delete", () => {
      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      });

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
        failOnStatusCode: false,
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { ...fixtures.blocklistCreateBody, type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("fingerprint_id", TEST_CARD_BIN);
        expect(response.body).to.have.property("data_kind", "card_bin");
      });

      cy.request({
        method: "POST",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("fingerprint_id", TEST_EXTENDED_BIN);
      });

      cy.request({
        method: "GET",
        url: `${getBaseUrl()}/blocklist?data_kind=card_bin`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("count").that.is.gte(1);
        expect(response.body.data).to.be.an("array");
      });

      cy.request({
        method: "GET",
        url: `${getBaseUrl()}/blocklist?data_kind=extended_card_bin`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("count").that.is.gte(1);
        expect(response.body.data).to.be.an("array");
      });

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "card_bin", data: TEST_CARD_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
      });

      cy.request({
        method: "DELETE",
        url: `${getBaseUrl()}/blocklist`,
        headers: getHeaders(),
        body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
      });

      cy.request({
        method: "GET",
        url: `${getBaseUrl()}/blocklist?data_kind=card_bin`,
        headers: getHeaders(),
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("data");
        expect(response.body.data).to.be.an("array");
        const match = response.body.data.find(
          (entry) => entry.fingerprint_id === TEST_CARD_BIN
        );
        expect(match).to.be.undefined;
      });
    });
  });
});
