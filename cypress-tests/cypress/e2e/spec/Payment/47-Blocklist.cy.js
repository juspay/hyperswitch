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
      let shouldContinue = true;

      cy.step("Create card_bin blocklist entry", () => {
        cy.blocklistCreateRule(
          fixtures.blocklistCreateBody,
          TEST_CARD_BIN,
          globalState
        );

        const data = connectorDetails.Blocklist.CreateCardBin;
        if (data.Response.body.error) {
          shouldContinue = false;
        }
      });

      cy.step("List card_bin entries and verify entry exists", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List card_bin entries");
          return;
        }
        cy.blocklistListRules("card_bin", globalState);

        const listData = connectorDetails.Blocklist.ListCardBin;
        if (listData.Response.body.error) {
          shouldContinue = false;
        }
      });

      cy.step("Delete card_bin entry", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Delete card_bin entry");
          return;
        }
        cy.blocklistDeleteRule("card_bin", TEST_CARD_BIN, globalState);
      });

      cy.step("Verify card_bin list is empty after deletion", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify empty list");
          return;
        }
        cy.blocklistListRules("card_bin", globalState);
      });
    });

    it("should reject duplicate card_bin blocklist entry", () => {
      let shouldContinue = true;

      cy.step("Ensure card_bin is not already blocked", () => {
        const apiKey = globalState.get("apiKey");
        const baseUrl = globalState.get("baseUrl");
        cy.request({
          method: "DELETE",
          url: `${baseUrl}/blocklist`,
          headers: { "api-key": apiKey, "Content-Type": "application/json" },
          body: { type: "card_bin", data: TEST_CARD_BIN },
          failOnStatusCode: false,
        });
      });

      cy.step("Create card_bin blocklist entry", () => {
        cy.blocklistCreateRule(
          fixtures.blocklistCreateBody,
          TEST_CARD_BIN,
          globalState
        );
      });

      cy.step("Attempt to create duplicate card_bin - should fail", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Duplicate creation");
          return;
        }
        const apiKey = globalState.get("apiKey");
        const baseUrl = globalState.get("baseUrl");
        cy.request({
          method: "POST",
          url: `${baseUrl}/blocklist`,
          headers: { "api-key": apiKey, "Content-Type": "application/json" },
          body: {
            type: "card_bin",
            data: TEST_CARD_BIN,
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.eq(400);
          expect(response.body.error.code).to.eq("IR_16");
        });
      });

      cy.step("Cleanup - delete card_bin entry", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Cleanup");
          return;
        }
        cy.blocklistDeleteRule("card_bin", TEST_CARD_BIN, globalState);
      });
    });
  });

  context("Extended Card BIN Blocklist Operations", () => {
    it("should add extended_card_bin to blocklist successfully", () => {
      let shouldContinue = true;

      cy.step("Create extended_card_bin blocklist entry", () => {
        const apiKey = globalState.get("apiKey");
        const baseUrl = globalState.get("baseUrl");
        const url = `${baseUrl}/blocklist`;

        cy.request({
          method: "POST",
          url: url,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: {
            type: "extended_card_bin",
            data: TEST_EXTENDED_BIN,
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.eq(200);
          expect(response.body).to.have.property("fingerprint_id", TEST_EXTENDED_BIN);
          expect(response.body).to.have.property("data_kind", "extended_card_bin");
        });
      });

      cy.step("List extended_card_bin entries", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List extended_card_bin entries");
          return;
        }
        cy.blocklistListRules("extended_card_bin", globalState);
      });

      cy.step("Delete extended_card_bin entry", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Delete extended_card_bin entry");
          return;
        }
        cy.blocklistDeleteRule("extended_card_bin", TEST_EXTENDED_BIN, globalState);
      });
    });
  });

  context("Full Blocklist Lifecycle", () => {
    it("should perform complete blocklist lifecycle - add all types, list, delete", () => {
      let shouldContinue = true;

      cy.step("Cleanup existing blocklist entries", () => {
        const apiKey = globalState.get("apiKey");
        const baseUrl = globalState.get("baseUrl");
        cy.request({
          method: "DELETE",
          url: `${baseUrl}/blocklist`,
          headers: { "api-key": apiKey, "Content-Type": "application/json" },
          body: { type: "card_bin", data: TEST_CARD_BIN },
          failOnStatusCode: false,
        });
        cy.request({
          method: "DELETE",
          url: `${baseUrl}/blocklist`,
          headers: { "api-key": apiKey, "Content-Type": "application/json" },
          body: { type: "extended_card_bin", data: TEST_EXTENDED_BIN },
          failOnStatusCode: false,
        });
      });

      cy.step("Add card_bin to blocklist", () => {
        cy.blocklistCreateRule(
          fixtures.blocklistCreateBody,
          TEST_CARD_BIN,
          globalState
        );

        const data = connectorDetails.Blocklist.CreateCardBin;
        if (data.Response.body.error) {
          shouldContinue = false;
        }
      });

      cy.step("Add extended_card_bin to blocklist", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Add extended_card_bin");
          return;
        }
        const apiKey = globalState.get("apiKey");
        const baseUrl = globalState.get("baseUrl");

        cy.request({
          method: "POST",
          url: `${baseUrl}/blocklist`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: {
            type: "extended_card_bin",
            data: TEST_EXTENDED_BIN,
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.eq(200);
        });
      });

      cy.step("List card_bin entries - should have 1", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List card_bin");
          return;
        }
        cy.blocklistListRules("card_bin", globalState);
      });

      cy.step("List extended_card_bin entries - should have 1", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List extended_card_bin");
          return;
        }
        cy.blocklistListRules("extended_card_bin", globalState);
      });

      cy.step("Delete card_bin entry", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Delete card_bin");
          return;
        }
        cy.blocklistDeleteRule("card_bin", TEST_CARD_BIN, globalState);
      });

      cy.step("Delete extended_card_bin entry", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Delete extended_card_bin");
          return;
        }
        cy.blocklistDeleteRule("extended_card_bin", TEST_EXTENDED_BIN, globalState);
      });

      cy.step("Verify card_bin list is empty", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify empty list");
          return;
        }
        cy.blocklistListRules("card_bin", globalState);
      });
    });
  });
});