import {access} from 'node:fs/promises';
import {constants} from 'node:fs';
import path from 'node:path';

/**
 * @param {{configured?: string, loadtestRoot?: string, consoleRoot?: string, pathValue?: string}} [options]
 */
export async function resolveK6Binary({configured=process.env.K6_BINARY,loadtestRoot,consoleRoot,pathValue=process.env.PATH || ''}={}) {
  const names=[];
  if(configured)names.push(path.resolve(configured));
  if(loadtestRoot)names.push(path.join(loadtestRoot,'k6/scenario-mix/bin/k6-sqlite'));
  if(consoleRoot)names.push(path.join(consoleRoot,'bin/k6-sqlite'));
  names.push('/usr/local/bin/k6-sqlite',...pathValue.split(path.delimiter).filter(Boolean).map(directory=>path.join(directory,'k6-sqlite')));
  for(const candidate of [...new Set(names)])try{await access(candidate,constants.X_OK);return candidate;}catch{}
  throw new Error('k6-sqlite not found. Run `npm run build:recorder` in loadtest/console or set K6_BINARY to the executable path.');
}
