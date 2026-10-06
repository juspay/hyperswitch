#!/usr/bin/env node
import { mkdir, readFile, writeFile, rename, access } from "node:fs/promises";
import { createHash, randomUUID } from "node:crypto";
import { spawn } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { loadConfig, provision, setFixtureRecorder } from "./provision-merchants.mjs";
import { FixtureRecorder } from "./sqlite.mjs";
import { configureSuperposition } from "./superposition.mjs";
import { reconcile } from "./reconcile.mjs";

const root = path.dirname(fileURLToPath(import.meta.url));
const digest = (text) => createHash("sha256").update(text).digest("hex");
async function atomicJson(file,value) {
  await writeFile(`${file}.tmp`,JSON.stringify(value,null,2),{ mode: 0o600 });
  await rename(`${file}.tmp`,file);
}

export async function fixtures(configPath,output) {
  const cfg = loadConfig(configPath);
  output = path.resolve(output);
  await mkdir(output,{ recursive: true, mode: 0o700 });
  const manifestFile = path.join(output,"manifest.json");
  cfg.output = path.join(output,"merchants.json");
  const identity = digest(JSON.stringify({ router:cfg.router,prefix:cfg.merchant_id_prefix,count:cfg.merchant_count,organization:cfg.organization_id,connector:cfg.connector }));
  let previous;
  try { previous = JSON.parse(await readFile(manifestFile,"utf8")); } catch (error) { if (error.code !== "ENOENT") throw error; }
  if (previous && previous.identity !== identity) throw new Error("Fixture provisioning identity changed; use a new output directory");
  const manifest = { schema_version: 1, identity, state: "creating", router: cfg.router };
  await atomicJson(manifestFile,manifest);
  const recorder = new FixtureRecorder(path.join(output,"fixtures.sqlite"),{ kind: "fixtures", identity });
  setFixtureRecorder(recorder);
  try {
    const result = await provision(cfg);
    const superposition = await configureSuperposition(cfg.superposition,recorder,result.merchants,result.organization_id);
    const merchantData = await readFile(cfg.output,"utf8");
    recorder.metadata.superposition = superposition;
    recorder.close();
    await atomicJson(manifestFile,{ ...manifest, state: "ready", ready_at: new Date().toISOString(),
      organization_id: result.organization_id, merchant_count: result.merchants.length,
      merchants_sha256: digest(merchantData), superposition });
    return { manifest: manifestFile, merchant_count: result.merchants.length };
  } catch (error) {
    try { recorder.close(error); } catch (_) { /* initial error is authoritative */ }
    await atomicJson(manifestFile,{ ...manifest,state: "failed",error: error.message });
    throw error;
  } finally { setFixtureRecorder(null); }
}

export async function load(configPath,fixturePath,output,binary = path.join(root,"bin/k6-sqlite"),{ recording = true, timing = false } = {}) {
  const config = JSON.parse(await readFile(configPath,"utf8"));
  fixturePath = path.resolve(fixturePath); output = path.resolve(output); binary = path.resolve(binary);
  const manifest = JSON.parse(await readFile(path.join(fixturePath,"manifest.json"),"utf8"));
  if (manifest.schema_version !== 1 || manifest.state !== "ready") throw new Error("Load requires a ready fixture manifest");
  const merchantsFile = path.join(fixturePath,"merchants.json");
  const merchants = await readFile(merchantsFile,"utf8");
  if (digest(merchants) !== manifest.merchants_sha256) throw new Error("Fixture credentials changed since readiness verification; rerun fixtures");
  if (config.services?.router?.replace(/\/$/,"") !== manifest.router) throw new Error("Load router must match fixture router");
  await access(binary);
  await mkdir(output,{ recursive: true,mode: 0o700 });
  const database = path.join(output,"results.sqlite");
  try { await access(database); throw new Error("Run database already exists; use a new output directory"); } catch (error) { if (error.code !== "ENOENT") throw error; }
  config.merchant_pool = { file: merchantsFile }; delete config.merchant;
  const generatedConfig = path.join(output,"load-config.json");
  await atomicJson(generatedConfig,config);
  // Preserve the original plain-k6 entrypoint. The recorded variant imports the native extension.
  const source = (await readFile(path.join(root,"scenario-mix.js"),"utf8"))
    .replace('"./recording-client.js"','"k6/x/sqlite-recorder"');
  const script = path.join(output,"scenario-mix.generated.js");
  await writeFile(script,source,{ mode: 0o600 });
  const runId = randomUUID();
  const metadata = { fixtures_sha256: digest(JSON.stringify(manifest)), load_config_sha256: digest(JSON.stringify(config)),
    superposition: manifest.superposition, load: config.load, scenarios: config.scenarios };
  const args = ["run",...(recording ? ["--out","sqlite-recorder"] : []),script];
  const child = spawn(timing ? "/usr/bin/time" : binary,timing ? ["-p",binary,...args] : args,{
    detached: process.platform !== "win32",
    stdio: timing ? ["inherit","inherit","pipe"] : "inherit", env: { ...process.env, RECORDING_DISABLED: recording ? "" : "1", SQLITE_RECORDER_REQUIRED: recording ? "1" : "",
      SCENARIO_MIX_CONFIG: generatedConfig, OUTPUT_DIR: output,SUMMARY_OUTPUT: "summary.json",
      SQLITE_RECORDER_DB: database,SQLITE_RECORDER_RUN_ID: runId,SQLITE_RECORDER_METADATA: JSON.stringify(metadata) },
  });
  let stderr = "";
  if (timing) child.stderr.on("data",(chunk) => { stderr += chunk; process.stderr.write(chunk); });
  const signals = ["SIGINT","SIGTERM"];
  const forward = (signal) => {
    try { if (process.platform === "win32") child.kill(signal); else process.kill(-child.pid,signal); }
    catch (error) { if (error.code !== "ESRCH") throw error; }
  };
  const handlers = signals.map((signal) => { const handler = () => forward(signal); process.on(signal,handler); return handler; });
  let code;
  try {
    code = await new Promise((resolve,reject) => { child.once("error",reject); child.once("close",(exitCode) => resolve(exitCode ?? 1)); });
  } finally { signals.forEach((signal,i) => process.removeListener(signal,handlers[i])); }
  const cpu = timing ? Object.fromEntries([...stderr.matchAll(/^(real|user|sys)\s+(\d+(?:\.\d+)?)$/gm)].map((m) => [m[1],Number(m[2])])) : null;
  if (!recording) return { run_id: runId, exitCode: code,cpu };
  const quote = (value) => `'${String(value).replaceAll("'", "'\\''")}'`;
  console.log(`\nReconcile this run (replace payments.csv with your CSV):\nnode ${quote(path.join(root,"run.mjs"))} reconcile --db ${quote(database)} --csv payments.csv --output ${quote(path.join(output,"reconciliation.csv"))}\n`);
  const db = new DatabaseSync(database,{ readOnly: true });
  try {
    const run = db.prepare("SELECT * FROM runs WHERE run_id=?").get(runId);
    if (!run || run.recording_state !== "complete" || run.enqueued !== run.persisted) throw new Error(`Run ${runId} has incomplete recording`);
  } finally { db.close(); }
  return { run_id: runId, database,exitCode: code,cpu };
}

function argumentsFor(command,args) {
  const allowed = { fixtures:["config","output"],load:["config","fixtures","output","k6"],
    reconcile:["db","csv","output","run-id","allow-incomplete"] }[command];
  if (!allowed) throw new Error("Usage: node run.mjs fixtures|load|reconcile --help");
  const values = {};
  for (let i=0;i<args.length;i++) {
    if (args[i] === "--help") return null;
    const key = args[i].replace(/^--/,"");
    if (!args[i].startsWith("--") || !allowed.includes(key) || key in values) throw new Error(`Invalid or duplicate argument: ${args[i]}`);
    if (key === "allow-incomplete") { values[key] = true; continue; }
    if (!args[i+1] || args[i+1].startsWith("--")) throw new Error(`Missing value for --${key}`);
    values[key] = args[++i];
  }
  const required = command === "fixtures" ? ["config","output"] : command === "load" ? ["config","fixtures","output"] : ["db","csv","output"];
  for (const key of required) if (!values[key]) throw new Error(`--${key} is required`);
  return values;
}
export async function main(args = process.argv.slice(2)) {
  const command = args[0]; const flags = argumentsFor(command,args.slice(1));
  if (!flags) {
    console.log("fixtures --config provision.json --output fixtures/\nload --config load.json --fixtures fixtures/ --output run/ [--k6 binary]\nreconcile --db run/results.sqlite --csv payments.csv --output reconciliation.csv [--run-id id] [--allow-incomplete]");
    return;
  }
  const result = command === "fixtures" ? await fixtures(flags.config,flags.output)
    : command === "load" ? await load(flags.config,flags.fixtures,flags.output,flags.k6)
    : await reconcile({ db:flags.db,csv:flags.csv,output:flags.output,runId:flags["run-id"],allowIncomplete:flags["allow-incomplete"] });
  console.log(JSON.stringify(result,null,2)); process.exitCode = result.exitCode || 0;
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => { console.error(error.stack || error.message); process.exitCode = 1; });
}
