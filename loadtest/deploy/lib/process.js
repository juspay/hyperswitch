"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const LOADTEST_ROOT = path.resolve(__dirname, "../..");
const DRY_RUN = ["1", "true"].includes(process.env.DRY_RUN);
let podmanEnvironment;

function helperDirectory() {
  const result = spawnSync("bash", ["-lc", 'readlink -f "$(command -v podman)"'], {
    encoding: "utf8",
    stdio: "pipe",
    env: process.env,
  });
  const binary = result.stdout.trim();
  if (!binary) return null;
  const candidate = path.resolve(path.dirname(binary), "../libexec/podman");
  return fs.existsSync(path.join(candidate, "rootlessport")) ? candidate : null;
}

function podmanEnv() {
  if (process.env.CONTAINERS_CONF) return {};
  if (podmanEnvironment) return podmanEnvironment;
  const helper = process.env.PODMAN_HELPER_BINARIES_DIR || helperDirectory();
  if (!helper) return (podmanEnvironment = {});
  const output = path.join(LOADTEST_ROOT, "deploy/generated/state/containers.conf");
  const escape = (value) => value.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  const profileBin = path.join(process.env.HOME || "", ".nix-profile/bin");
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, `[engine]\nhelper_binaries_dir=["${escape(helper)}","${escape(profileBin)}"]\n`);
  return (podmanEnvironment = { CONTAINERS_CONF: output });
}

function run(command, args, options = {}) {
  const printable = [command, ...args].join(" ");
  if (DRY_RUN) {
    console.log(`[dry-run] ${printable}`);
    return { status: 0, stdout: "", stderr: "" };
  }
  const result = spawnSync(command, args, {
    cwd: options.cwd || LOADTEST_ROOT,
    encoding: "utf8",
    input: options.input,
    stdio: options.capture || options.input !== undefined ? "pipe" : "inherit",
    env: { ...process.env, ...(command === "podman" ? podmanEnv() : {}), ...(options.env || {}) },
  });
  if (result.error) throw result.error;
  if (result.status !== 0 && !options.allowFailure) throw new Error(`Command failed: ${printable}`);
  return result;
}

function runShell(command, options = {}) {
  return run("bash", ["-lc", command], options);
}

module.exports = { run, runShell };
