import customerCreateBody from "../../fixtures/create-customer-body.json";
import State from "../../utils/State";

let globalState;

describe("Customer Create flow test", () => {

    before("seed global state", () => {

        cy.task('getGlobalState').then((state) => {
            globalState = new State(state);
            console.log("seeding globalState -> " + JSON.stringify(globalState));
        })
    })
    after("flush global state", () => {
        console.log("flushing globalState -> " + JSON.stringify(globalState));
        cy.task('setGlobalState', globalState.data);
    })
    it("customer-create-call-test", () => {

        cy.createCustomerCallTest(customerCreateBody, globalState);

    });
}); 