import {publicSettings,dataRoot} from '../../../../server/state.mjs';
import {listCertificates} from '../../../../server/certificates.mjs';
import {listWorkers} from '../../../../server/worker-registry.mjs';
export function load({depends}){depends('console:settings');return {settings:publicSettings(),certificates:listCertificates(dataRoot),workers:listWorkers()};}
