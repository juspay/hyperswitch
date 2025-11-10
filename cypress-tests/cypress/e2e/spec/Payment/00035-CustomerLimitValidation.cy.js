import State from "../../../utils/State";

let globalState;

describe("Customer List With Count - Limit Validation flow test", () => {
	before("seed global state", () => {
		cy.task("getGlobalState").then((state) => {
			globalState = new State(state);
		});
	});

	after("flush global state", () => {
		cy.task("setGlobalState", globalState.data);
	});

	it("customer-list-limit-validation-call-test", () => {
		cy.customerListLimitValidationCallTest(globalState);
	});
});
