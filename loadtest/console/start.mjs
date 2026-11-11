// The SSR process does not own or stop worker jobs.
// Artifact uploads are streamed and bounded by their route. Other request bodies
// retain a 1 MiB limit in hooks, including chunked requests and login forms.
process.env.BODY_SIZE_LIMIT ??= '2000000000';
import {initializeSupervisor} from './server/supervisor.mjs';
import {startGrafanaProxy} from './server/grafana-embed.mjs';
if(!process.env.ADMIN_PASSWORD_HASH)throw new Error('ADMIN_PASSWORD_HASH is required; run npm run password to generate it');
await initializeSupervisor();
const grafanaProxy=await startGrafanaProxy();
const closeGrafana=()=>{grafanaProxy.closeAllConnections();grafanaProxy.close();};
process.once('SIGTERM',closeGrafana);
process.once('SIGINT',closeGrafana);
await import('./build/index.js');
