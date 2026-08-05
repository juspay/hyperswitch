#!/usr/bin/env node
"use strict";

const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");
const { loadYaml, resolveMaybe } = require("../../lib/config");

const LOADTEST_ROOT = path.resolve(__dirname, "../..");
const CONFIG_PATH = path.resolve(LOADTEST_ROOT, process.env.CONFIG || "deploy/config.yaml");
const EXAMPLE_CONFIG_PATH = path.resolve(LOADTEST_ROOT, "deploy/config.example.yaml");
const DRY_RUN = process.env.DRY_RUN === "1" || process.env.DRY_RUN === "true";
const FORCE = process.env.FORCE === "1" || process.env.FORCE === "true";
let PODMAN_ENV_CACHE = null;

function loadConfig() {
  const configPath = fs.existsSync(CONFIG_PATH) ? CONFIG_PATH : EXAMPLE_CONFIG_PATH;
  if (!fs.existsSync(configPath)) throw new Error(`Config file not found: ${CONFIG_PATH}`);
  if (configPath === EXAMPLE_CONFIG_PATH && !process.env.CONFIG) console.log("Using deploy/config.example.yaml because deploy/config.yaml does not exist.");
  return { config: loadYaml(configPath), configDir: path.dirname(configPath), configPath };
}

function entries(section) {
  return Object.entries(section || {}).filter(([, item]) => item && item.enabled !== false);
}

function stateServices(config) {
  return entries(config.state_services);
}

function applicationServices(config) {
  return entries(config.application_services);
}

function managedServices(config) {
  return [...stateServices(config), ...applicationServices(config)];
}

function selectedServices(state, names) {
  if (!names.length) return managedServices(state.config);
  return names.map((name) => {
    const match = managedServices(state.config).find(([serviceName]) => serviceName === name);
    if (!match) throw new Error(`Unknown managed service: ${name}`);
    return match;
  });
}

function repos(config) {
  return Object.entries(config.repositories || {}).filter(([, repo]) => repo && repo.enabled !== false);
}

function commandParts(command) {
  if (!command) return [];
  return String(command).split(/\s+/).filter(Boolean);
}

function arrayValue(value) {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

function run(command, args, options = {}) {
  const printable = [command, ...args].join(" ");
  if (DRY_RUN) {
    console.log(`[dry-run] ${printable}`);
    return { status: 0, stdout: "", stderr: "" };
  }
  const commandEnv = command === "podman" ? podmanEnv() : {};
  const result = spawnSync(command, args, {
    cwd: options.cwd || LOADTEST_ROOT,
    encoding: "utf8",
    input: options.input,
    stdio: options.capture || options.input !== undefined ? "pipe" : "inherit",
    env: { ...process.env, ...commandEnv, ...(options.env || {}) },
  });
  if (result.error) throw result.error;
  if (result.status !== 0 && !options.allowFailure) throw new Error(`Command failed: ${printable}`);
  return result;
}

function podmanEnv() {
  if (process.env.CONTAINERS_CONF) return {};
  if (PODMAN_ENV_CACHE) return PODMAN_ENV_CACHE;

  const helperDir = process.env.PODMAN_HELPER_BINARIES_DIR || detectPodmanHelperDir();
  if (!helperDir) {
    PODMAN_ENV_CACHE = {};
    return PODMAN_ENV_CACHE;
  }

  const output = path.join(LOADTEST_ROOT, "deploy/generated/state/containers.conf");
  const escapedHelperDir = helperDir.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  const escapedProfileBin = path.join(process.env.HOME || "", ".nix-profile/bin")
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"');
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(
    output,
    `[engine]\nhelper_binaries_dir=["${escapedHelperDir}","${escapedProfileBin}"]\n`,
  );
  PODMAN_ENV_CACHE = { CONTAINERS_CONF: output };
  return PODMAN_ENV_CACHE;
}

function detectPodmanHelperDir() {
  const result = spawnSync("bash", ["-lc", 'readlink -f "$(command -v podman)"'], {
    encoding: "utf8",
    stdio: "pipe",
    env: process.env,
  });
  const podmanPath = result.stdout.trim();
  if (!podmanPath) return null;
  const candidate = path.resolve(path.dirname(podmanPath), "../libexec/podman");
  if (fs.existsSync(path.join(candidate, "rootlessport"))) return candidate;
  return null;
}

function runShell(command, options = {}) {
  return run("bash", ["-lc", command], options);
}

function ensureDir(dir) {
  if (DRY_RUN) {
    console.log(`[dry-run] mkdir -p ${dir}`);
    return;
  }
  fs.mkdirSync(dir, { recursive: true });
}

function writeFileOnce(file, contents, secret = false) {
  if (!FORCE && fs.existsSync(file)) {
    console.log(`${secret ? "secret " : ""}exists: ${file}`);
    return;
  }
  if (DRY_RUN) {
    console.log(`[dry-run] write ${secret ? "secret " : ""}${file}`);
    return;
  }
  ensureDir(path.dirname(file));
  fs.writeFileSync(file, contents);
  if (secret) fs.chmodSync(file, 0o600);
  console.log(`${secret ? "secret " : ""}wrote: ${file}`);
}

function writeGeneratedFile(file, contents, secret = false) {
  if (DRY_RUN) {
    console.log(`[dry-run] write ${secret ? "secret " : ""}${file}`);
    return;
  }
  ensureDir(path.dirname(file));
  fs.writeFileSync(file, contents);
  if (secret) fs.chmodSync(file, 0o600);
  console.log(`${secret ? "secret " : ""}wrote: ${file}`);
}

function generatedRoot(state) {
  return resolveMaybe(state.configDir, state.config.prepare?.output_dir || "generated");
}

function logRoot(config) {
  return config.logs?.root || "logs";
}

function serviceLogDir(state, name, service) {
  return resolveMaybe(state.configDir, path.join(logRoot(state.config), service.log_dir || name));
}

function repoPath(state, name) {
  const repo = state.config.repositories?.[name];
  if (!repo || !repo.path) return null;
  return resolveMaybe(state.configDir, repo.path);
}

function tokenMap(state) {
  const postgres = state.config.state_services?.postgres || {};
  const redis = state.config.state_services?.redis || {};
  const postgresEnv = postgres.env || {};
  const schemas = postgres.schemas || {};
  const prefixes = redis.prefixes || {};
  const apps = state.config.application_services || {};
  const root = generatedRoot(state);
  const hyperswitchRepoPath = repoPath({ config: state.config, configDir: state.configDir }, "hyperswitch") || LOADTEST_ROOT;
  const databaseUrl = (override) => {
    if (override) return override;
    return `postgres://${postgresEnv.POSTGRES_USER || "postgres"}:${postgresEnv.POSTGRES_PASSWORD || "postgres"}@${postgres.host || "127.0.0.1"}:${postgres.port || "5432"}/${postgres.database || postgresEnv.POSTGRES_DB || "hyperswitch"}`;
  };
  const tokens = {
    POSTGRES_HOST: postgres.host || "127.0.0.1",
    POSTGRES_PORT: postgres.port || "5432",
    POSTGRES_DB: postgres.database || postgresEnv.POSTGRES_DB || "hyperswitch",
    POSTGRES_USER: postgresEnv.POSTGRES_USER || "postgres",
    POSTGRES_PASSWORD: postgresEnv.POSTGRES_PASSWORD || "postgres",
    POSTGRES_SCHEMA_ROUTER: schemas.router || schemas.payments || "public",
    POSTGRES_SCHEMA_PAYMENTS: schemas.payments || "router",
    POSTGRES_SCHEMA_MODULAR: schemas.modular || schemas.payments || "router",
    POSTGRES_SCHEMA_VAULT: schemas.vault || "vault",
    POSTGRES_SCHEMA_ENCRYPTION: schemas.encryption || "encryption",
    POSTGRES_SCHEMA_SUPERPOSITION: schemas.superposition || "superposition",
    PAYMENTS_DATABASE_URL: databaseUrl(postgres.payments_database_url),
    MODULAR_DATABASE_URL: databaseUrl(postgres.modular_database_url),
    VAULT_DATABASE_URL: databaseUrl(postgres.vault_database_url),
    ENCRYPTION_DATABASE_URL: databaseUrl(postgres.encryption_database_url),
    SUPERPOSITION_DATABASE_URL: databaseUrl(postgres.superposition_database_url),
    REDIS_HOST: redis.host || "127.0.0.1",
    REDIS_PORT: redis.port || "6379",
    REDIS_PREFIX_PAYMENTS: prefixes.payments || "router",
    REDIS_PREFIX_MODULAR: prefixes.modular || "modular",
    REDIS_PREFIX_VAULT: prefixes.vault || "vault",
    REDIS_PREFIX_ENCRYPTION: prefixes.encryption || "encryption",
    REDIS_PREFIX_SUPERPOSITION: prefixes.superposition || "superposition",
    PAYMENTS_BASE_URL: apps.payments?.base_url || "http://127.0.0.1:8080",
    MODULAR_BASE_URL: apps.modular?.base_url || "http://127.0.0.1:8081",
    VAULT_BASE_URL: apps.vault?.base_url || "http://127.0.0.1:3001",
    ENCRYPTION_BASE_URL: apps.encryption?.base_url || "http://127.0.0.1:5000",
    SUPERPOSITION_BASE_URL: apps.superposition?.base_url || "http://127.0.0.1:8082",
    GENERATED_ROOT: root,
    HYPERSWITCH_REPO_PATH: hyperswitchRepoPath,
    VAULT_PRIVATE_KEY_PATH: path.join(root, "keys/vault_private_key.pem"),
    VAULT_PUBLIC_KEY_PATH: path.join(root, "keys/vault_public_key.pem"),
    TENANT_MASTER_KEY_PATH: path.join(root, "keys/tenant_master_key.hex"),
    DEPLOY_CONFIG_DIR: state.configDir,
  };
  for (const [name] of repos(state.config)) {
    const repo = repoPath(state, name);
    if (repo) tokens[`REPO_${name.toUpperCase().replace(/[^A-Z0-9]+/g, "_")}`] = repo;
  }
  return tokens;
}

function render(contents, tokens) {
  return contents.replace(/\{\{([A-Z0-9_]+)\}\}/g, (match, key) => String(tokens[key] ?? match));
}

function serviceRunArgs(state, name, service) {
  const args = [
    "run",
    "--replace",
    "--detach",
    "--name",
    service.container,
    "--label",
    "io.hyperswitch.loadtest.managed=true",
    "--label",
    `io.hyperswitch.loadtest.service=${name}`,
  ];
  args.push("--network", service.network || "host");
  if (service.cpuset !== null && service.cpuset !== undefined && service.cpuset !== "") args.push("--cpuset-cpus", String(service.cpuset));
  if (service.user !== null && service.user !== undefined && service.user !== "") args.push("--user", String(service.user));
  for (const [key, value] of Object.entries(service.env || {})) if (value !== null && value !== undefined) args.push("-e", `${key}=${value}`);
  if (service.env_file) args.push("--env-file", render(resolveMaybe(state.configDir, service.env_file), tokenMap(state)));
  for (const volume of arrayValue(service.volumes).filter(Boolean)) args.push("-v", render(String(volume), tokenMap(state)));
  if (service.log_mount) {
    const hostLogDir = serviceLogDir(state, name, service);
    ensureDir(hostLogDir);
    args.push("-v", `${hostLogDir}:${service.log_mount}`);
  }
  if (service.ports && (service.network || "host") !== "host") for (const port of arrayValue(service.ports)) args.push("-p", String(port));
  args.push(service.image);
  if (service.command) args.push(...commandParts(render(service.command, tokenMap(state))));
  return args;
}

function reposCommand(state) {
  for (const [name, repo] of repos(state.config)) {
    const target = resolveMaybe(state.configDir, repo.path);
    if (fs.existsSync(target)) {
      console.log(`${name}: using ${target}`);
      continue;
    }
    if (repo.git) {
      ensureDir(path.dirname(target));
      console.log(`${name}: cloning ${repo.git} -> ${target}`);
      const args = ["clone"];
      if (repo.ref) args.push("--branch", String(repo.ref));
      args.push(repo.git, target);
      run("git", args);
      continue;
    }
    if (repo.required !== false) throw new Error(`${name}: repository path missing and git URL not configured: ${target}`);
    console.log(`${name}: optional repository missing: ${target}`);
  }
}

function build(state) {
  reposCommand(state);
  for (const [name, service] of applicationServices(state.config)) {
    if (!service.build_enabled) {
      console.log(`${name}: build disabled`);
      continue;
    }
    if (service.build_command) {
      const cwd = repoPath(state, service.build_repo) || resolveMaybe(state.configDir, service.build_context || "../..");
      console.log(`${name}: ${service.build_command}`);
      runShell(render(service.build_command, tokenMap(state)), { cwd });
      continue;
    }
    if (!service.dockerfile) throw new Error(`${name}: build_enabled is true but dockerfile/build_command is missing`);
    run("podman", ["build", "--format", "docker", "-t", service.image, "-f", resolveMaybe(state.configDir, service.dockerfile), resolveMaybe(state.configDir, service.build_context || "../..")]);
  }
}

function generateKeys(state) {
  if (state.config.prepare?.generate_keys === false) return;
  const root = generatedRoot(state);
  const privateKeyPath = path.join(root, "keys/vault_private_key.pem");
  const publicKeyPath = path.join(root, "keys/vault_public_key.pem");
  const masterKeyPath = path.join(root, "keys/tenant_master_key.hex");
  if (FORCE || !fs.existsSync(privateKeyPath) || !fs.existsSync(publicKeyPath)) {
    const { privateKey, publicKey } = crypto.generateKeyPairSync("rsa", { modulusLength: Number(state.config.prepare?.rsa_bits || 2048) });
    writeFileOnce(privateKeyPath, privateKey.export({ type: "pkcs1", format: "pem" }), true);
    writeFileOnce(publicKeyPath, publicKey.export({ type: "spki", format: "pem" }), false);
  } else {
    console.log(`secret exists: ${privateKeyPath}`);
    console.log(`exists: ${publicKeyPath}`);
  }
  if (FORCE || !fs.existsSync(masterKeyPath)) writeFileOnce(masterKeyPath, `${crypto.randomBytes(32).toString("hex")}\n`, true);
  else console.log(`secret exists: ${masterKeyPath}`);
}

function prepareConfigs(state) {
  const configs = state.config.prepare?.configs || {};
  const tokens = tokenMap(state);
  for (const [name, item] of Object.entries(configs)) {
    if (!item || item.enabled === false) continue;
    if (!item.output) throw new Error(`prepare.configs.${name}.output is required`);
    const output = path.join(generatedRoot(state), item.output);
    const inputPath = item.template ? resolveMaybe(state.configDir, item.template) : resolveMaybe(state.configDir, item.source);
    if (!inputPath) throw new Error(`prepare.configs.${name}: template or source is required`);
    if (!fs.existsSync(inputPath)) throw new Error(`prepare.configs.${name}: input not found: ${inputPath}`);
    writeFileOnce(output, render(fs.readFileSync(inputPath, "utf8"), tokens));
  }
}

function prepareStateSql(state) {
  const root = generatedRoot(state);
  const output = path.join(root, "state/init.sql");
  const dieselConfigOutput = path.join(root, "state/diesel-migrations-only.toml");
  const postgres = state.config.state_services?.postgres || {};
  const schemas = [...new Set(Object.values(postgres.schemas || {}).filter(Boolean))];
  const sql = [
    ...schemas.map((schema) => `CREATE SCHEMA IF NOT EXISTS ${schema};`),
    "",
  ].join("\n");
  writeFileOnce(output, sql);
  writeGeneratedFile(dieselConfigOutput, "# Generated for loadtest service migrations. Intentionally omits [print_schema].\n");
}

function readGeneratedSecret(state, relativePath) {
  const file = path.join(generatedRoot(state), relativePath);
  if (!fs.existsSync(file)) throw new Error(`Missing generated secret: ${file}`);
  return fs.readFileSync(file, "utf8").trim();
}

function generatedSecretTokens(state) {
  return {
    ...tokenMap(state),
    VAULT_PRIVATE_KEY: readGeneratedSecret(state, "keys/vault_private_key.pem"),
    VAULT_PUBLIC_KEY: readGeneratedSecret(state, "keys/vault_public_key.pem"),
    TENANT_MASTER_KEY: readGeneratedSecret(state, "keys/tenant_master_key.hex"),
  };
}

function overridePath(state, name, fallback) {
  const overrides = state.config.prepare?.overrides || {};
  return resolveMaybe(state.configDir, overrides[name] || fallback);
}

function prepareBuiltinServiceConfigs(state) {
  const apps = state.config.application_services || {};
  const root = generatedRoot(state);
  const tokens = generatedSecretTokens(state);
  if (apps.vault?.enabled !== false) {
    const template = fs.readFileSync(overridePath(state, "vault", "overrides/vault.toml"), "utf8");
    writeGeneratedFile(path.join(root, "configs/vault.toml"), render(template, tokens), true);
  }
  if (apps.encryption?.enabled !== false) {
    const encryptionConfig = render(
      fs.readFileSync(overridePath(state, "encryption", "overrides/encryption.toml"), "utf8"),
      tokens,
    );
    writeGeneratedFile(path.join(root, "configs/encryption/development.toml"), encryptionConfig, true);
    writeGeneratedFile(path.join(root, "configs/encryption/Dev.toml"), encryptionConfig, true);
  }
}

function replaceMultilineTomlValue(contents, key, value) {
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const replacement = `${key} = """\n${value}\n"""`;
  return contents.replace(new RegExp(`${escapedKey}\\s*=\\s*"""[\\s\\S]*?"""`), replacement);
}

function replaceTomlSectionString(contents, section, key, value) {
  const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*)"[^"]*"`);
  if (!pattern.test(contents)) throw new Error(`Missing ${section}.${key} in router config`);
  return contents.replace(pattern, `$1"${value}"`);
}

function replaceTomlSectionBoolean(contents, section, key, value) {
  const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*)(true|false)`);
  if (!pattern.test(contents)) throw new Error(`Missing ${section}.${key} in router config`);
  return contents.replace(pattern, `$1${value}`);
}

function replaceTomlSectionNumber(contents, section, key, value) {
  const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*)-?[0-9]+(?:\\.[0-9]+)?`);
  if (!pattern.test(contents)) throw new Error(`Missing ${section}.${key} in router config`);
  return contents.replace(pattern, `$1${value}`);
}

function prepareRouterVaultKeys(state) {
  const tokens = generatedSecretTokens(state);
  const override = render(
    fs.readFileSync(overridePath(state, "router_jwekey", "overrides/router-jwekey.toml"), "utf8"),
    tokens,
  );
  const valueFor = (key) => {
    const match = override.match(new RegExp(`${key}\\s*=\\s*"""([\\s\\S]*?)"""`));
    if (!match) throw new Error(`Missing ${key} in router-jwekey override`);
    return match[1].trim();
  };
  const stringValueFor = (section, key) => {
    const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = override.match(new RegExp(`\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*"([^"]+)"`));
    if (!match) throw new Error(`Missing ${section}.${key} in router override`);
    return match[1];
  };
  const booleanValueFor = (section, key) => {
    const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = override.match(new RegExp(`\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*(true|false)`));
    if (!match) throw new Error(`Missing ${section}.${key} in router override`);
    return match[1];
  };
  const numberValueFor = (section, key) => {
    const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = override.match(new RegExp(`\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*(-?[0-9]+(?:\\.[0-9]+)?)`));
    if (!match) throw new Error(`Missing ${section}.${key} in router override`);
    return match[1];
  };
  for (const file of [
    path.join(generatedRoot(state), "configs/payments.toml"),
    path.join(generatedRoot(state), "configs/modular.toml"),
  ]) {
    if (!fs.existsSync(file)) continue;
    let contents = fs.readFileSync(file, "utf8");
    contents = replaceMultilineTomlValue(contents, "vault_encryption_key", valueFor("vault_encryption_key"));
    contents = replaceMultilineTomlValue(contents, "rust_locker_encryption_key", valueFor("rust_locker_encryption_key"));
    contents = replaceMultilineTomlValue(contents, "vault_private_key", valueFor("vault_private_key"));
    contents = replaceTomlSectionString(contents, "log.console", "level", stringValueFor("log.console", "level"));
    contents = replaceTomlSectionString(contents, "log.console", "log_format", stringValueFor("log.console", "log_format"));
    contents = replaceTomlSectionBoolean(contents, "log.telemetry", "traces_enabled", booleanValueFor("log.telemetry", "traces_enabled"));
    contents = replaceTomlSectionBoolean(contents, "log.telemetry", "metrics_enabled", booleanValueFor("log.telemetry", "metrics_enabled"));
    contents = replaceTomlSectionNumber(contents, "dummy_connector", "payment_duration", numberValueFor("dummy_connector", "payment_duration"));
    contents = replaceTomlSectionNumber(contents, "dummy_connector", "payment_tolerance", numberValueFor("dummy_connector", "payment_tolerance"));
    writeGeneratedFile(file, contents, true);
  }
}

function prepare(state) {
  reposCommand(state);
  ensureDir(generatedRoot(state));
  ensureDir(resolveMaybe(state.configDir, logRoot(state.config)));
  generateKeys(state);
  prepareConfigs(state);
  prepareBuiltinServiceConfigs(state);
  prepareRouterVaultKeys(state);
  prepareStateSql(state);
  console.log(`generated: ${generatedRoot(state)}`);
}

function compose(state, args) {
  const obs = state.config.observability || {};
  const parts = commandParts(obs.compose_command || "podman compose");
  const composeFile = resolveMaybe(state.configDir, obs.compose_file || "../docker-compose.yaml");
  const prometheusUrl = new URL(obs.prometheus_url || "http://127.0.0.1:9090");
  const lokiUrl = new URL(obs.loki_url || "http://127.0.0.1:3100");
  run(parts[0], [...parts.slice(1), "-f", composeFile, "-p", obs.project_name || "hyperswitch-loadtest", ...args], {
    env: {
      PROMETHEUS_URL: prometheusUrl.origin,
      PROMETHEUS_PORT: prometheusUrl.port || "9090",
      LOKI_URL: lokiUrl.origin,
      HOST_UID: String(process.getuid?.() ?? 1000),
      HOST_GID: String(process.getgid?.() ?? 1000),
    },
  });
}

function ensurePodmanSocket() {
  const runtimeDir = process.env.XDG_RUNTIME_DIR || `/run/user/${process.getuid()}`;
  const socketPath = path.join(runtimeDir, "podman/podman.sock");
  fs.mkdirSync(path.dirname(socketPath), { recursive: true });
  if (fs.existsSync(socketPath) && fs.statSync(socketPath).isDirectory()) {
    fs.rmSync(socketPath, { recursive: true, force: true });
  }
  const socketUnit = run("systemctl", ["--user", "start", "podman.socket"], { allowFailure: true, capture: true });
  if (socketUnit.status !== 0) {
    const existing = run("systemctl", ["--user", "start", "hs-loadtest-podman-api.service"], { allowFailure: true, capture: true });
    if (existing.status !== 0) {
      const podmanPath = run("which", ["podman"], { capture: true }).stdout.trim();
      run("systemctl", ["--user", "reset-failed", "hs-loadtest-podman-api.service"], { allowFailure: true, capture: true });
      run("systemd-run", [
        "--user",
        "--collect",
        "--unit=hs-loadtest-podman-api",
        "--property=Restart=on-failure",
        podmanPath,
        "system",
        "service",
        "--time=0",
        `unix://${socketPath}`,
      ]);
    }
  }
  waitUntil("podman API socket", () => {
    const result = run("curl", ["--silent", "--show-error", "--unix-socket", socketPath, "http://d/_ping"], {
      allowFailure: true,
      capture: true,
    });
    return result.status === 0;
  });
}

function upState(state) {
  ensureDir(resolveMaybe(state.configDir, logRoot(state.config)));
  for (const [name, service] of stateServices(state.config)) {
    console.log(`Starting state ${name} as ${service.container} on CPU ${service.cpuset || "unconfigured"}`);
    run("podman", serviceRunArgs(state, name, service));
  }
}

function upApps(state) {
  for (const [name, service] of applicationServices(state.config)) {
    console.log(`Starting app ${name} as ${service.container} on CPU ${service.cpuset || "unconfigured"}`);
    run("podman", serviceRunArgs(state, name, service));
  }
}

function upObservability(state) {
  if ((state.config.observability || {}).enabled !== false) {
    ensurePodmanSocket();
    compose(state, ["up", "-d", "--force-recreate"]);
  }
}

function up(state) {
  upState(state);
  upApps(state);
  upObservability(state);
}

function waitUntil(name, probe, attempts = 60) {
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    if (probe()) {
      console.log(`${name}: ready`);
      return;
    }
    if (attempt < attempts) run("sleep", ["1"]);
  }
  throw new Error(`${name}: did not become ready within ${attempts} seconds`);
}

function waitForState(state) {
  const postgres = state.config.state_services?.postgres;
  if (postgres?.enabled !== false) {
    waitUntil("postgres", () => run("podman", [
      "exec",
      postgres.container,
      "pg_isready",
      "-p",
      String(postgres.port || "5432"),
      "-U",
      postgres.env?.POSTGRES_USER || "postgres",
      "-d",
      postgres.env?.POSTGRES_DB || "hyperswitch",
    ], { capture: true, allowFailure: true }).status === 0);
  }

  const redis = state.config.state_services?.redis;
  if (redis?.enabled !== false) {
    waitUntil("redis", () => run("podman", [
      "exec",
      redis.container,
      "redis-cli",
      "-p",
      String(redis.port || "6379"),
      "ping",
    ], { capture: true, allowFailure: true }).status === 0);
  }
}

function urlIsReady(url) {
  const result = run("curl", ["-fsS", "-o", "/dev/null", "-w", "%{http_code}", url], { capture: true, allowFailure: true });
  return result.status === 0 && /^2\d\d$/.test(result.stdout.trim());
}

function waitForHttpServices(state) {
  for (const [name, service] of applicationServices(state.config)) {
    if (service.health_url) waitUntil(name, () => urlIsReady(service.health_url));
  }

  const obs = state.config.observability || {};
  if (obs.enabled === false) return;
  if (obs.loki_url) waitUntil("loki", () => urlIsReady(`${obs.loki_url.replace(/\/$/, "")}/ready`));
  if (obs.prometheus_url) waitUntil("prometheus", () => urlIsReady(`${obs.prometheus_url.replace(/\/$/, "")}/-/ready`));
  if (obs.grafana_url) waitUntil("grafana", () => urlIsReady(`${obs.grafana_url.replace(/\/$/, "")}/api/health`));
}

function initState(state) {
  const postgres = state.config.state_services?.postgres;
  if (postgres?.enabled !== false) {
    const sqlPath = path.join(generatedRoot(state), "state/init.sql");
    if (!fs.existsSync(sqlPath)) throw new Error(`Missing generated state SQL: ${sqlPath}. Run deploy-prepare first.`);
    const sql = fs.readFileSync(sqlPath, "utf8");
    run("podman", [
      "exec",
      "-i",
      postgres.container,
      "psql",
      "-p",
      String(postgres.port || "5432"),
      "-U",
      postgres.env?.POSTGRES_USER || "postgres",
      "-d",
      postgres.env?.POSTGRES_DB || "hyperswitch",
    ], { input: sql });
  }
  const migrationNames = process.argv.slice(3).filter(Boolean);
  const migrations = entries(state.config.migrations).filter(([name]) => !migrationNames.length || migrationNames.includes(name));
  for (const [name, migration] of migrations) {
    if (!migration.command) {
      console.log(`${name}: migration command not configured`);
      continue;
    }
    const cwd = repoPath(state, migration.repo);
    if (!cwd) throw new Error(`${name}: migration repo path missing: ${migration.repo}`);
    const env = {};
    for (const [key, value] of Object.entries(migration.env || {})) env[key] = render(String(value), tokenMap(state));
    console.log(`${name}: ${migration.command}`);
    runShell(render(migration.command, tokenMap(state)), { cwd, env });
  }
}

function down(state) {
  if ((state.config.observability || {}).enabled !== false) compose(state, ["down"]);
  for (const [, service] of applicationServices(state.config).reverse()) run("podman", ["stop", service.container], { allowFailure: true });
  for (const [, service] of stateServices(state.config).reverse()) run("podman", ["stop", service.container], { allowFailure: true });
}

function resetState(state) {
  for (const [, service] of stateServices(state.config).reverse()) {
    run("podman", ["rm", "-f", service.container], { allowFailure: true });
  }
  for (const [name, service] of stateServices(state.config)) {
    for (const mount of service.volumes || []) {
      const source = String(mount).split(":", 1)[0];
      if (!source || source.startsWith("/") || source.startsWith(".")) continue;
      console.log(`Removing ${name} state volume ${source}`);
      run("podman", ["volume", "rm", "-f", source], { allowFailure: true });
    }
  }
}

function restart(state) {
  const names = process.argv.slice(3).filter(Boolean);
  const selected = selectedServices(state, names);
  for (const [, service] of selected.slice().reverse()) run("podman", ["stop", service.container], { allowFailure: true });
  for (const [name, service] of selected) {
    const type = stateServices(state.config).some(([serviceName]) => serviceName === name) ? "state" : "app";
    console.log(`Starting ${type} ${name} as ${service.container} on CPU ${service.cpuset || "unconfigured"}`);
    run("podman", serviceRunArgs(state, name, service));
  }
  waitForHttpServices(state);
}

function containerLine(service) {
  const ps = run("podman", ["ps", "-a", "--filter", `name=^${service.container}$`, "--format", "{{.Names}}\t{{.Status}}\t{{.Image}}"], { capture: true, allowFailure: true });
  return ps.stdout.trim() || "not created";
}

function containerAffinity(service) {
  const pid = run("podman", ["inspect", "--format", "{{.State.Pid}}", service.container], { capture: true, allowFailure: true }).stdout.trim();
  if (!pid || pid === "0") return "n/a";
  return run("taskset", ["-pc", pid], { capture: true, allowFailure: true }).stdout.trim() || "unknown";
}

function status(state) {
  console.log(`Config: ${state.configPath}`);
  for (const [name, service] of stateServices(state.config)) {
    console.log(`state/${name}: ${containerLine(service)}`);
    console.log(`  expected CPU: ${service.cpuset || "unconfigured"}; ${containerAffinity(service)}`);
  }
  for (const [name, service] of applicationServices(state.config)) {
    console.log(`app/${name}: ${containerLine(service)}`);
    console.log(`  expected CPU: ${service.cpuset || "unconfigured"}; ${containerAffinity(service)}`);
  }
}

function checkUrl(name, url) {
  const result = run("curl", ["-fsS", "-o", "/dev/null", "-w", "%{http_code}", url], { capture: true, allowFailure: true });
  const status = result.stdout.trim() || "failed";
  console.log(`${name}: ${url} -> ${status}`);
  if (result.status !== 0 || !/^2\d\d$/.test(status)) {
    throw new Error(`${name}: health check failed for ${url} (${status})`);
  }
}

function smoke(state) {
  for (const [name, service] of managedServices(state.config)) if (service.health_url) checkUrl(name, service.health_url); else console.log(`${name}: no health_url configured`);
  const obs = state.config.observability || {};
  if (obs.enabled !== false) {
    if (obs.loki_url) checkUrl("loki", `${obs.loki_url.replace(/\/$/, "")}/ready`);
    if (obs.prometheus_url) checkUrl("prometheus", `${obs.prometheus_url.replace(/\/$/, "")}/-/ready`);
    if (obs.grafana_url) checkUrl("grafana", `${obs.grafana_url.replace(/\/$/, "")}/api/health`);
  }
}

function preflight(state) {
  run("podman", ["--version"], { capture: true });
  const cpus = run("nproc", [], { capture: true }).stdout.trim();
  console.log(`Config: ${state.configPath}`);
  console.log(`Logical CPUs visible: ${cpus}`);
  for (const [name, service] of managedServices(state.config)) {
    if (!service.container) throw new Error(`${name}: container is required`);
    if (!service.image) throw new Error(`${name}: image is required`);
    if (service.build_enabled && !service.dockerfile && !service.build_command) throw new Error(`${name}: dockerfile or build_command is required when build_enabled=true`);
    console.log(`${name}: ok`);
  }
  for (const [name, repo] of repos(state.config)) {
    const target = repo.path ? resolveMaybe(state.configDir, repo.path) : null;
    console.log(`repo/${name}: ${target || repo.git || "unconfigured"}`);
  }
}

function logs(state) {
  const serviceName = process.env.SERVICE || process.argv[3];
  if (!serviceName) throw new Error("Set SERVICE=<name> or pass service name after logs");
  const match = managedServices(state.config).find(([name]) => name === serviceName);
  if (!match) throw new Error(`Unknown managed service: ${serviceName}`);
  const [, service] = match;
  const args = ["logs", "--tail", process.env.TAIL || "200"];
  if (process.env.FOLLOW === "1" || process.env.FOLLOW === "true") args.push("-f");
  args.push(service.container);
  run("podman", args);
}

function observabilityLogs(state) {
  const service = process.env.SERVICE || process.argv[3];
  if (!service) throw new Error("Set SERVICE=<name> or pass an observability service name after observability-logs");
  compose(state, ["logs", "--tail", process.env.TAIL || "200", service]);
}

function restartObservability(state) {
  const service = process.env.SERVICE || process.argv[3];
  if (!service) throw new Error("Set SERVICE=<name> or pass an observability service name after restart-observability");
  compose(state, ["restart", service]);
  const obs = state.config.observability || {};
  const healthUrls = {
    loki: obs.loki_url ? `${obs.loki_url.replace(/\/$/, "")}/ready` : null,
    prometheus: obs.prometheus_url ? `${obs.prometheus_url.replace(/\/$/, "")}/-/ready` : null,
    grafana: obs.grafana_url ? `${obs.grafana_url.replace(/\/$/, "")}/api/health` : null,
  };
  if (healthUrls[service]) waitUntil(service, () => urlIsReady(healthUrls[service]));
}

function logPaths(state) {
  console.log(`root: ${resolveMaybe(state.configDir, logRoot(state.config))}`);
  for (const [name, service] of managedServices(state.config)) console.log(`${name}: ${serviceLogDir(state, name, service)}`);
}

function ready(state) {
  reposCommand(state);
  build(state);
  prepare(state);
  upState(state);
  waitForState(state);
  initState(state);
  upApps(state);
  upObservability(state);
  waitForHttpServices(state);
  runHooks(state, "post_start");
  smoke(state);
}

function runHooks(state, section) {
  for (const [name, hook] of entries(state.config[section])) {
    if (hook.enabled === false || !hook.command) continue;
    const cwd = hook.repo ? repoPath(state, hook.repo) : state.configDir;
    if (!cwd) throw new Error(`${section} ${name}: repository path missing: ${hook.repo}`);
    const env = {};
    for (const [key, value] of Object.entries(hook.env || {})) env[key] = render(String(value), tokenMap(state));
    console.log(`${section} ${name}: ${hook.command}`);
    runShell(render(hook.command, tokenMap(state)), { cwd, env });
  }
}

function main() {
  const command = process.argv[2] || "status";
  const state = loadConfig();
  if (command === "preflight") return preflight(state);
  if (command === "repos") return reposCommand(state);
  if (command === "build") return build(state);
  if (command === "prepare") return prepare(state);
  if (command === "up-state") return upState(state);
  if (command === "init-state") return initState(state);
  if (command === "up-apps") return upApps(state);
  if (command === "up-observability") return upObservability(state);
  if (command === "up") return up(state);
  if (command === "down") return down(state);
  if (command === "reset-state") return resetState(state);
  if (command === "restart") return restart(state);
  if (command === "ready") return ready(state);
  if (command === "status") return status(state);
  if (command === "smoke") return smoke(state);
  if (command === "logs") return logs(state);
  if (command === "observability-logs") return observabilityLogs(state);
  if (command === "restart-observability") return restartObservability(state);
  if (command === "log-paths") return logPaths(state);
  throw new Error(`Unknown deploy command: ${command}`);
}

try {
  main();
} catch (error) {
  console.error(`error: ${error.message}`);
  process.exit(1);
}
