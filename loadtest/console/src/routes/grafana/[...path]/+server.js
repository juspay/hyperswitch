// Active Grafana content must never be served on the console origin.
const isolated=()=>Response.json({error:'Grafana panels use the isolated embed origin'},{status:403});
export const GET=isolated,HEAD=isolated,POST=isolated;
