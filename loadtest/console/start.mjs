// The SSR process does not own or stop worker jobs.
import {initializeSupervisor} from './server/supervisor.mjs';
if(!process.env.ADMIN_PASSWORD_HASH)throw new Error('ADMIN_PASSWORD_HASH is required; run npm run password to generate it');
await initializeSupervisor();
await import('./build/index.js');
