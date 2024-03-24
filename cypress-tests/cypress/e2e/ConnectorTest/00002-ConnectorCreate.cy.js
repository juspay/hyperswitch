import createConnectorBody from "../../fixtures/create-connector-body.json";
import State from "../../utils/State";
// import ConnectorAuthDetails from "../../../creds.json";
let globalState;
describe("Connector Account Create flow test", () => {

  before("seed global state", () => {

    cy.task('getGlobalState').then((state) => {
      // visit non same-origin url https://www.cypress-dx.com
      globalState = new State(state);
      console.log("seeding globalState -> " + JSON.stringify(globalState));
    })
  })

  after("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task('setGlobalState', globalState.data);
  })

  it("connector-create-call-test", () => {
    // Provide the authType directly, no need to use getValueByKey
    // const authType = "nmi";
    // const authDetails = getValueByKey(ConnectorAuthDetails, authType);

    // Check if the authType exists in the creds.json file
    // if (authDetails) {
    // Use the specified authType details from creds.json
    cy.createConnectorCallTest(createConnectorBody, globalState);
    // } else {
    //   // Handle the case where the specified authType is not found
    //   throw new Error(`Authentication details not found for ${authType}`);
    // }
  });
});
