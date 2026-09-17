import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("3DS Routing Region for Unified Authentication Service", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.AUTH_SERVICE_ELIGIBILITY
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("threeds_routing_region_uas = Region1", () => {
    before("set org config to Region1", () => {
      const orgId = globalState.get("organizationId");
      cy.setupConfigs(
        globalState,
        `threeds_routing_region_uas_${orgId}`,
        "Region1"
      );
    });

    it("should confirm 3DS payment with UAS routing region set to Region1", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .threeds_routing_region_uas.Region1;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    after("cleanup org config", () => {
      const orgId = globalState.get("organizationId");
      cy.setConfigs(
        globalState,
        `threeds_routing_region_uas_${orgId}`,
        "Region1",
        "DELETE"
      );
    });
  });

  context("threeds_routing_region_uas = Region2", () => {
    before("set org config to Region2", () => {
      const orgId = globalState.get("organizationId");
      cy.setupConfigs(
        globalState,
        `threeds_routing_region_uas_${orgId}`,
        "Region2"
      );
    });

    it("should confirm 3DS payment with UAS routing region set to Region2", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .threeds_routing_region_uas.Region2;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    after("cleanup org config", () => {
      const orgId = globalState.get("organizationId");
      cy.setConfigs(
        globalState,
        `threeds_routing_region_uas_${orgId}`,
        "Region2",
        "DELETE"
      );
    });
  });

  context(
    "threeds_routing_region_uas set to an invalid value - falls back to default region",
    () => {
      before("set org config to an invalid value", () => {
        const orgId = globalState.get("organizationId");
        cy.setupConfigs(
          globalState,
          `threeds_routing_region_uas_${orgId}`,
          "NotARegion"
        );
      });

      it("should still confirm 3DS payment, falling back to the default region on parse failure", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))
          .threeds_routing_region_uas.InvalidRegion;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      after("cleanup org config", () => {
        const orgId = globalState.get("organizationId");
        cy.setConfigs(
          globalState,
          `threeds_routing_region_uas_${orgId}`,
          "NotARegion",
          "DELETE"
        );
      });
    }
  );

  context("No threeds_routing_region_uas config set - default behavior", () => {
    it("should confirm 3DS payment using the default UAS routing region (Region1)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .threeds_routing_region_uas.NoConfigDefault;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });
  });
});
