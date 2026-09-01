#!/usr/bin/env node
"use strict";

const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { loadYaml, resolveMaybe } = require("../../lib/config");
const { run, runShell } = require("./process");

const LOADTEST_ROOT = path.resolve(__dirname, "../..");
const CONFIG_PATH = path.resolve(LOADTEST_ROOT, process.env.CONFIG || "deploy/config.yaml");
const EXAMPLE_CONFIG_PATH = path.resolve(LOADTEST_ROOT, "deploy/config.example.yaml");
const DRY_RUN = process.env.DRY_RUN === "1" || process.env.DRY_RUN === "true";
const FORCE = process.env.FORCE === "1" || process.env.FORCE === "true";

function loadConfig() {
  const configPath = fs.existsSync(CONFIG_PATH) ? CONFIG_PATH : EXAMPLE_CONFIG_PATH;
  if (!fs.existsSync(configPath)) throw new Error(`Config file not found: ${CONFIG_PATH}`);
  if (configPath === EXAMPLE_CONFIG_PATH && !process.env.CONFIG) console.log("Using deploy/config.example.yaml because deploy/config.yaml does not exist.");
  const config = loadYaml(configPath);
  for (const [name, service] of Object.entries(config.application_services || {})) {
    service.container = `hs-${name}`;
  }
  return { config, configDir: path.dirname(configPath), configPath };
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

function sourceEntries(config) {
  return [
    ...applicationServices(config),
    ...entries(config.external_dependencies),
  ].filter(([, item]) => item.source);
}

function sourceConfig(config, name) {
  return config.application_services?.[name]?.source
    || config.external_dependencies?.[name]?.source
    || null;
}

function commandParts(command) {
  if (!command) return [];
  return String(command).split(/\s+/).filter(Boolean);
}

function arrayValue(value) {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
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
  const source = sourceConfig(state.config, name);
  if (!source?.path) return null;
  return resolveMaybe(state.configDir, source.path);
}

function requiredConfig(value, key) {
  if (value === undefined || value === null || value === "") throw new Error(`${key} is required`);
  return value;
}

// Deployment YAML is the source of truth. These values only bridge it into
// generated application TOMLs, container arguments, and migration commands.
function templateVariables(state) {
  const postgres = state.config.state_services?.postgres || {};
  const redis = state.config.state_services?.redis || {};
  const postgresEnv = postgres.env || {};
  const schemas = postgres.schemas || {};
  const databaseUrls = postgres.database_urls || {};
  const prefixes = redis.prefixes || {};
  const apps = state.config.application_services || {};
  const root = generatedRoot(state);
  const hyperswitchRepoPath = requiredConfig(repoPath(state, "router"), "application_services.router.source.path");
  const databaseUrl = (override) => {
    if (override) return override;
    return `postgres://${requiredConfig(postgresEnv.POSTGRES_USER, "state_services.postgres.env.POSTGRES_USER")}:${requiredConfig(postgresEnv.POSTGRES_PASSWORD, "state_services.postgres.env.POSTGRES_PASSWORD")}@${requiredConfig(postgres.host, "state_services.postgres.host")}:${requiredConfig(postgres.port, "state_services.postgres.port")}/${requiredConfig(postgres.database, "state_services.postgres.database")}`;
  };
  const variables = {
    POSTGRES_HOST: requiredConfig(postgres.host, "state_services.postgres.host"),
    POSTGRES_PORT: requiredConfig(postgres.port, "state_services.postgres.port"),
    POSTGRES_DB: requiredConfig(postgres.database, "state_services.postgres.database"),
    POSTGRES_USER: requiredConfig(postgresEnv.POSTGRES_USER, "state_services.postgres.env.POSTGRES_USER"),
    POSTGRES_PASSWORD: requiredConfig(postgresEnv.POSTGRES_PASSWORD, "state_services.postgres.env.POSTGRES_PASSWORD"),
    POSTGRES_SCHEMA_ROUTER: requiredConfig(schemas.router, "state_services.postgres.schemas.router"),
    POSTGRES_SCHEMA_MODULAR_PM: requiredConfig(schemas["modular-pm"], "state_services.postgres.schemas.modular-pm"),
    POSTGRES_SCHEMA_VAULT: requiredConfig(schemas.vault, "state_services.postgres.schemas.vault"),
    POSTGRES_SCHEMA_ENCRYPTION: requiredConfig(schemas.encryption, "state_services.postgres.schemas.encryption"),
    POSTGRES_SCHEMA_SUPERPOSITION: requiredConfig(schemas.superposition, "state_services.postgres.schemas.superposition"),
    ROUTER_DATABASE_URL: databaseUrl(databaseUrls.router),
    MODULAR_PM_DATABASE_URL: databaseUrl(databaseUrls["modular-pm"]),
    VAULT_DATABASE_URL: databaseUrl(databaseUrls.vault),
    ENCRYPTION_DATABASE_URL: databaseUrl(databaseUrls.encryption),
    SUPERPOSITION_DATABASE_URL: databaseUrl(databaseUrls.superposition),
    REDIS_HOST: requiredConfig(redis.host, "state_services.redis.host"),
    REDIS_PORT: requiredConfig(redis.port, "state_services.redis.port"),
    REDIS_PREFIX_ROUTER: requiredConfig(prefixes.router, "state_services.redis.prefixes.router"),
    REDIS_PREFIX_MODULAR_PM: requiredConfig(prefixes["modular-pm"], "state_services.redis.prefixes.modular-pm"),
    REDIS_PREFIX_VAULT: requiredConfig(prefixes.vault, "state_services.redis.prefixes.vault"),
    REDIS_PREFIX_ENCRYPTION: requiredConfig(prefixes.encryption, "state_services.redis.prefixes.encryption"),
    REDIS_PREFIX_SUPERPOSITION: requiredConfig(prefixes.superposition, "state_services.redis.prefixes.superposition"),
    ROUTER_BASE_URL: requiredConfig(apps.router?.base_url, "application_services.router.base_url"),
    MODULAR_PM_BASE_URL: requiredConfig(apps["modular-pm"]?.base_url, "application_services.modular-pm.base_url"),
    VAULT_BASE_URL: requiredConfig(apps.vault?.base_url, "application_services.vault.base_url"),
    ENCRYPTION_BASE_URL: requiredConfig(apps.encryption?.base_url, "application_services.encryption.base_url"),
    SUPERPOSITION_BASE_URL: requiredConfig(apps.superposition?.base_url, "application_services.superposition.base_url"),
    GENERATED_ROOT: root,
    HYPERSWITCH_REPO_PATH: hyperswitchRepoPath,
    VAULT_PRIVATE_KEY_PATH: path.join(root, "keys/vault_private_key.pem"),
    VAULT_PUBLIC_KEY_PATH: path.join(root, "keys/vault_public_key.pem"),
    TENANT_MASTER_KEY_PATH: path.join(root, "keys/tenant_master_key.hex"),
    DEPLOY_CONFIG_DIR: state.configDir,
  };
  for (const [name] of sourceEntries(state.config)) {
    const repo = repoPath(state, name);
    if (repo) variables[`REPO_${name.toUpperCase().replace(/[^A-Z0-9]+/g, "_")}`] = repo;
  }
  return variables;
}

function render(contents, variables) {
  return contents.replace(/\{\{([A-Z0-9_]+)\}\}/g, (_, key) => String(requiredConfig(variables[key], `template variable ${key}`)));
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
  if (service.env_file) args.push("--env-file", render(resolveMaybe(state.configDir, service.env_file), templateVariables(state)));
  for (const volume of arrayValue(service.volumes).filter(Boolean)) args.push("-v", render(String(volume), templateVariables(state)));
  if (service.log_mount) {
    const hostLogDir = serviceLogDir(state, name, service);
    ensureDir(hostLogDir);
    args.push("-v", `${hostLogDir}:${service.log_mount}`);
  }
  if (service.ports && (service.network || "host") !== "host") for (const port of arrayValue(service.ports)) args.push("-p", String(port));
  args.push(service.image);
  if (service.command) args.push(...commandParts(render(service.command, templateVariables(state))));
  return args;
}

function reposCommand(state) {
  const preparedPaths = new Set();
  for (const [name, item] of sourceEntries(state.config)) {
    const source = item.source;
    const target = resolveMaybe(state.configDir, source.path);
    if (preparedPaths.has(target)) {
      console.log(`${name}: using shared repository ${target}`);
      continue;
    }
    preparedPaths.add(target);
    run("bash", [
      path.join(__dirname, "../scripts/repository.sh"),
      target,
      source.git_url,
      source.ref ? String(source.ref) : "",
    ]);
  }
}

function imageExists(image) {
  return run("podman", ["image", "exists", image], { capture: true, allowFailure: true }).status === 0;
}

function build(state) {
  reposCommand(state);
  for (const [name, service] of applicationServices(state.config)) {
    const source = service.source;
    if (source.mode === "cloud") {
      console.log(`${name}: pulling ${service.image}`);
      run("podman", ["pull", service.image]);
      continue;
    }
    const buildConfig = service.build || {};
    if (!FORCE && !buildConfig.force && imageExists(service.image)) {
      console.log(`${name}: using local image ${service.image}`);
      continue;
    }
    if (buildConfig.command) {
      console.log(`${name}: ${buildConfig.command}`);
      runShell(render(buildConfig.command, templateVariables(state)), { cwd: repoPath(state, name) });
      continue;
    }
    if (!buildConfig.dockerfile) throw new Error(`${name}: local source requires build.command or build.dockerfile`);
    run("podman", [
      "build",
      "--format",
      "docker",
      "-t",
      service.image,
      "-f",
      resolveMaybe(state.configDir, buildConfig.dockerfile),
      resolveMaybe(state.configDir, buildConfig.context || source.path),
    ]);
  }
}

function generateKeys(state) {
  const keyConfig = state.config.prepare?.vault?.keys || {};
  if (keyConfig.generate === false) return;
  const root = generatedRoot(state);
  const privateKeyPath = path.join(root, "keys/vault_private_key.pem");
  const publicKeyPath = path.join(root, "keys/vault_public_key.pem");
  const masterKeyPath = path.join(root, "keys/tenant_master_key.hex");
  if (FORCE || !fs.existsSync(privateKeyPath) || !fs.existsSync(publicKeyPath)) {
    const { privateKey, publicKey } = crypto.generateKeyPairSync("rsa", { modulusLength: Number(keyConfig.rsa_bits || 2048) });
    writeFileOnce(privateKeyPath, privateKey.export({ type: "pkcs8", format: "pem" }), true);
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
  const variables = templateVariables(state);
  for (const [name, item] of Object.entries(configs)) {
    if (!item || item.enabled === false) continue;
    if (!item.output) throw new Error(`prepare.configs.${name}.output is required`);
    const output = path.join(generatedRoot(state), item.output);
    const inputPath = item.template ? resolveMaybe(state.configDir, item.template) : resolveMaybe(state.configDir, item.source);
    if (!inputPath) throw new Error(`prepare.configs.${name}: template or source is required`);
    if (!fs.existsSync(inputPath)) throw new Error(`prepare.configs.${name}: input not found: ${inputPath}`);
    writeFileOnce(output, render(fs.readFileSync(inputPath, "utf8"), variables));
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
    ...templateVariables(state),
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
  const pattern = new RegExp(
    `${escapedKey}\\s*=\\s*(?:"""[\\s\\S]*?"""|"[^"\\n]*")`,
  );
  if (!pattern.test(contents)) return contents;
  return contents.replace(pattern, replacement);
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

function setTomlSectionNumber(contents, section, key, value) {
  const header = `[${section}]`;
  const sectionStart = contents.indexOf(header);
  if (sectionStart < 0) throw new Error(`Missing ${section} section in router config`);
  const nextSection = contents.indexOf("\n[", sectionStart + header.length);
  const sectionEnd = nextSection < 0 ? contents.length : nextSection;
  const sectionContents = contents.slice(sectionStart, sectionEnd);
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(^|\\n)(${escapedKey}\\s*=\\s*)-?[0-9]+(?:\\.[0-9]+)?`);
  if (pattern.test(sectionContents)) {
    const updatedSection = sectionContents.replace(pattern, `$1$2${value}`);
    return `${contents.slice(0, sectionStart)}${updatedSection}${contents.slice(sectionEnd)}`;
  }
  const insertAt = sectionStart + header.length;
  return `${contents.slice(0, insertAt)}\n${key} = ${value}${contents.slice(insertAt)}`;
}

function setTomlSectionString(contents, section, key, value) {
  const header = `[${section}]`;
  const sectionStart = contents.indexOf(header);
  if (sectionStart < 0) throw new Error(`Missing ${section} section in router config`);
  const nextSection = contents.indexOf("\n[", sectionStart + header.length);
  const sectionEnd = nextSection < 0 ? contents.length : nextSection;
  const sectionContents = contents.slice(sectionStart, sectionEnd);
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(^|\\n)(${escapedKey}\\s*=\\s*)"[^"]*"`);
  if (pattern.test(sectionContents)) {
    const updatedSection = sectionContents.replace(pattern, `$1$2"${value}"`);
    return `${contents.slice(0, sectionStart)}${updatedSection}${contents.slice(sectionEnd)}`;
  }
  const insertAt = sectionStart + header.length;
  return `${contents.slice(0, insertAt)}\n${key} = "${value}"${contents.slice(insertAt)}`;
}

function prepareRouterVaultKeys(state) {
  const tokens = generatedSecretTokens(state);
  const applyOverride = (file, overrideName, fallback) => {
    if (!fs.existsSync(file)) return;
    const override = render(
      fs.readFileSync(overridePath(state, overrideName, fallback), "utf8"),
      tokens,
    );
    const valueFor = (key) => {
      const match = override.match(new RegExp(`${key}\\s*=\\s*"""([\\s\\S]*?)"""`));
      if (!match) throw new Error(`Missing ${key} in ${overrideName} override`);
      return match[1].trim();
    };
    const stringValueFor = (section, key) => {
    const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = override.match(new RegExp(`\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*"([^"]+)"`));
    if (!match) throw new Error(`Missing ${section}.${key} in ${overrideName} override`);
    return match[1];
    };
    const booleanValueFor = (section, key) => {
    const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = override.match(new RegExp(`\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*(true|false)`));
    if (!match) throw new Error(`Missing ${section}.${key} in ${overrideName} override`);
    return match[1];
    };
    const numberValueFor = (section, key) => {
    const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const match = override.match(new RegExp(`\\[${escapedSection}\\][\\s\\S]*?\\n${escapedKey}\\s*=\\s*(-?[0-9]+(?:\\.[0-9]+)?)`));
    if (!match) throw new Error(`Missing ${section}.${key} in ${overrideName} override`);
    return match[1];
    };
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
    contents = setTomlSectionString(contents, "server", "host", stringValueFor("server", "host"));
    for (const section of ["master_database", "replica_database", "accounts_database", "global_database"]) {
      contents = setTomlSectionString(contents, section, "username", tokens.POSTGRES_USER);
      contents = setTomlSectionString(contents, section, "password", tokens.POSTGRES_PASSWORD);
      contents = setTomlSectionString(contents, section, "host", tokens.POSTGRES_HOST);
      contents = setTomlSectionNumber(contents, section, "port", tokens.POSTGRES_PORT);
      contents = setTomlSectionString(contents, section, "dbname", tokens.POSTGRES_DB);
    }
    contents = setTomlSectionString(contents, "redis", "host", tokens.REDIS_HOST);
    contents = setTomlSectionNumber(contents, "redis", "port", tokens.REDIS_PORT);
    contents = setTomlSectionString(contents, "locker", "host", tokens.VAULT_BASE_URL);
    contents = replaceTomlSectionBoolean(
      contents,
      "internal_merchant_id_profile_id_auth",
      "enabled",
      "true",
    );
    contents = setTomlSectionString(
      contents,
      "micro_services",
      "payment_methods_base_url",
      tokens.MODULAR_PM_BASE_URL,
    );
    if (overrideName === "router") {
      contents = replaceTomlSectionBoolean(contents, "locker", "mock_locker", booleanValueFor("locker", "mock_locker"));
      contents = replaceTomlSectionBoolean(
        contents,
        "micro_services",
        "use_legacy_locker",
        booleanValueFor("micro_services", "use_legacy_locker"),
      );
      contents = setTomlSectionString(
        contents,
        "superposition",
        "endpoint",
        stringValueFor("superposition", "endpoint"),
      );
      contents = setTomlSectionNumber(
        contents,
        "superposition",
        "polling_interval",
        numberValueFor("superposition", "polling_interval"),
      );
    }
    if (overrideName === "modular-pm") {
      contents = setTomlSectionNumber(contents, "server", "port", numberValueFor("server", "port"));
    }
    writeGeneratedFile(file, contents, true);
  };
  applyOverride(path.join(generatedRoot(state), "configs/router.toml"), "router", "overrides/router.toml");
  applyOverride(path.join(generatedRoot(state), "configs/modular-pm.toml"), "modular-pm", "overrides/modular-pm.toml");
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
    const sourceName = migration.source || name;
    const cwd = repoPath(state, sourceName);
    if (!cwd) throw new Error(`${name}: migration source path missing: ${sourceName}`);
    const env = {};
    for (const [key, value] of Object.entries(migration.env || {})) env[key] = render(String(value), templateVariables(state));
    console.log(`${name}: ${migration.command}`);
    runShell(render(migration.command, templateVariables(state)), { cwd, env });
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
  templateVariables(state);
  const cpus = run("nproc", [], { capture: true }).stdout.trim();
  console.log(`Config: ${state.configPath}`);
  console.log(`Logical CPUs visible: ${cpus}`);
  for (const [name, service] of managedServices(state.config)) {
    if (!service.container) throw new Error(`${name}: container is required`);
    if (!service.image) throw new Error(`${name}: image is required`);
    if (state.config.application_services?.[name]) {
      const source = service.source;
      if (!source?.path || !source?.git_url) throw new Error(`${name}: source.path and source.git_url are required`);
      if (!["local", "cloud"].includes(source.mode)) throw new Error(`${name}: source.mode must be local or cloud`);
    }
    console.log(`${name}: ok`);
  }
  for (const [name, item] of sourceEntries(state.config)) {
    const target = resolveMaybe(state.configDir, item.source.path);
    console.log(`source/${name}: ${item.source.mode || "dependency"} ${target}`);
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
    const sourceName = hook.source;
    const cwd = sourceName ? repoPath(state, sourceName) : state.configDir;
    if (!cwd) throw new Error(`${section} ${name}: source path missing: ${sourceName}`);
    const env = {};
    for (const [key, value] of Object.entries(hook.env || {})) env[key] = render(String(value), templateVariables(state));
    console.log(`${section} ${name}: ${hook.command}`);
    runShell(render(hook.command, templateVariables(state)), { cwd, env });
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
