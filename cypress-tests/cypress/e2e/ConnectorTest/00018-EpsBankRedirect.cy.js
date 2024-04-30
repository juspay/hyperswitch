import getConnectorDetails from "../ConnectorUtils/utils";
import confirmBody from "../../fixtures/confirm-body.json";

import State from "../../utils/State";
import createPaymentBody from "../../fixtures/create-payment-body.json";

let globalState;

describe("Eps Bank Redirect flow test", () => {

	before("seed global state", () => {

		cy.task('getGlobalState').then((state) => {
			globalState = new State(state);
			console.log("seeding globalState -> " + JSON.stringify(globalState));
		})
	})

	afterEach("flush global state", () => {
		console.log("flushing globalState -> " + JSON.stringify(globalState));
		cy.task('setGlobalState', globalState.data);
	})

	context("Eps Create and Confirm flow test", () => {

		it("create-payment-call-test", () => {
			let det = getConnectorDetails(globalState.get("connectorId"))["BankRedirect"]["3DS"]["eps"];
			cy.createPaymentIntentTest(createPaymentBody, det, "three_ds", "automatic", globalState);
		});

		it("confirm-call-test", () => {
			let det = getConnectorDetails(globalState.get("connectorId"))["BankRedirect"]["3DS"]["eps"];
			cy.task('cli_log', "GLOBAL STATE -> " + JSON.stringify(globalState.data));
			cy.confirmCallTestBankRedirect(confirmBody, det, true, globalState);
		});

		it("handle-redirection-call-test", () => {
			let expected_redirection = confirmBody["return_url"];
            globalState.set("test-confirm", "success");
			cy.handleRedirectionBankRedirect(globalState, expected_redirection);
		});

	});

});