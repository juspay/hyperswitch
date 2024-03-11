import merchantCreateBody from "../../fixtures/merchant-create-body.json";
import apiKeyCreateBody from "../../fixtures/create-api-key-body.json";
import State from "../../utils/State";

let globalState;
describe("Account Create flow test", () => {

  before("seed global state",  () => {
    
    cy.task('getGlobalState').then((state) => {
      globalState = new State(state);
      console.log("seeding globalState -> "+JSON.stringify(globalState));
    })
  })
  after("flush global state", () => {
    console.log("flushing globalState -> "+ JSON.stringify(globalState));
    cy.task('setGlobalState', globalState.data);
  })

  it("merchant-create-call-test", () => {
    cy.merchantCreateCallTest(merchantCreateBody, globalState);
  });
  it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
  });
  
});
