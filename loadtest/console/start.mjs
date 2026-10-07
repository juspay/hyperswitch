// Register worker shutdown before adapter-node's signal handlers close the HTTP server.
import { initializeJobs,shutdownJobs } from './server/jobs.mjs';
initializeJobs();
if(!process.env.ADMIN_PASSWORD_HASH)throw new Error('ADMIN_PASSWORD_HASH is required; run npm run password to generate it');
process.env.SHUTDOWN_TIMEOUT ||= '70';
await import('./build/index.js');
process.on('sveltekit:shutdown',shutdownJobs);
