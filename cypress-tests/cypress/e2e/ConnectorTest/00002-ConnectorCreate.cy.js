import createConnectorBody from "../../fixtures/create-connector-body.json";
import State from "../../utils/State";
// import ConnectorAuthDetails from "../../../creds.json";
let globalState;
describe("Connector Account Create flow test", () => {

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

  it("connector-create-call-test", () => {
    cy.createConnectorCallTest(createConnectorBody, globalState);
  });
});
