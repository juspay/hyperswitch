import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("[Payout] Profile List Count Validation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("verify-total-count-field-exists-in-profile-list", () => {
    const baseUrl = globalState.get("baseUrl");
    const apiKey = globalState.get("apiKey");
    const profileId = globalState.get("profileId");

    cy.request({
      method: "GET",
      url: `${baseUrl}/payouts/profile/list`,
      headers: {
        "Content-Type": "application/json",
        "api-key": apiKey,
      },
      qs: {
        profile_id: profileId,
        limit: 10,
      },
    }).then((response) => {
      expect(response.status).to.eq(200);
      expect(response.body).to.have.property("data");
      expect(response.body).to.have.property("total_count");
      expect(response.body.total_count).to.be.a("number");
      expect(response.body.total_count).to.be.at.least(0);
    });
  });

  it("verify-total-count-matches-data-length-for-empty-profile", () => {
    const baseUrl = globalState.get("baseUrl");
    const apiKey = globalState.get("apiKey");

    cy.request({
      method: "GET",
      url: `${baseUrl}/payouts/profile/list`,
      headers: {
        "Content-Type": "application/json",
        "api-key": apiKey,
      },
      qs: {
        limit: 10,
      },
    }).then((response) => {
      expect(response.status).to.eq(200);
      expect(response.body).to.have.property("data");
      expect(response.body).to.have.property("total_count");

      const actualDataLength = response.body.data.length;
      const totalCount = response.body.total_count;

      expect(totalCount).to.be.a("number");

      if (actualDataLength === 0) {
        expect(totalCount).to.equal(0);
      } else {
        expect(totalCount).to.be.at.least(actualDataLength);
      }
    });
  });

  it("verify-merchant-level-list-returns-total-count", () => {
    const baseUrl = globalState.get("baseUrl");
    const apiKey = globalState.get("apiKey");

    cy.request({
      method: "POST",
      url: `${baseUrl}/payouts/list`,
      headers: {
        "Content-Type": "application/json",
        "api-key": apiKey,
      },
      body: {
        limit: 10,
      },
    }).then((response) => {
      expect(response.status).to.eq(200);
      expect(response.body).to.have.property("data");
      expect(response.body).to.have.property("total_count");
      expect(response.body.total_count).to.not.be.null;
      expect(response.body.total_count).to.be.a("number");
      expect(response.body.total_count).to.be.at.least(0);
    });
  });
});
