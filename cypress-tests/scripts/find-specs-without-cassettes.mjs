#!/usr/bin/env node
/**
 * Finds Payment spec files that have no recorded cassettes for any of the
 * given connectors.  Used by CI to split specs into two runs:
 *   - live run  (no mitmproxy): specs with no cassettes + setup specs
 *   - replay run (mitmproxy):   remaining specs
 *
 * Usage:
 *   node find-specs-without-cassettes.mjs \
 *     --connectors "redsys worldpayxml cybersource braintree" \
 *     --capture-dir /path/to/captures \
 *     --spec-dir    cypress/e2e/spec/Payment
 *
 * Outputs (stdout): space-separated list of spec file paths with no cassettes,
 * or nothing if all specs have cassettes.
 */

import { readFileSync, existsSync, readdirSync } from "fs";
import { join, relative } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Mirror of stableHashHex in cypress/support/e2e.js — must stay in sync.
function stableHashHex(input) {
  let hash = 5381;
  for (let i = 0; i < input.length; i++) {
    hash = ((hash * 33) ^ input.charCodeAt(i)) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function hasAnyCassette(hash, captureDir) {
  const search = (dir) => {
    if (!existsSync(dir)) return false;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        if (search(join(dir, entry.name))) return true;
      } else if (entry.name.startsWith(`${hash}-`)) {
        return true;
      }
    }
    return false;
  };
  return search(captureDir);
}

/**
 * Extract all test title paths from a spec file.
 * Returns strings like "Describe > Context > it title".
 * Handles single-level nesting (describe > context > it) which all Payment
 * specs use. Falls back gracefully for unusual structures.
 */
function extractTitlePaths(specContent) {
  const paths = [];
  const titleStack = [];

  for (const line of specContent.split("\n")) {
    // Push: describe/context/it titles
    const blockMatch = line.match(
      /^\s*(describe|context|it)\s*\(\s*["'`]([^"'`\n]+)["'`]/
    );
    if (!blockMatch) continue;

    const [, kind, title] = blockMatch;

    if (kind === "describe") {
      titleStack.length = 0;
      titleStack.push(title);
    } else if (kind === "context") {
      titleStack.length = Math.min(titleStack.length, 1); // keep describe
      titleStack.push(title);
    } else {
      // it
      const path = [...titleStack, title].join(" > ");
      paths.push(path);
    }
  }

  return paths;
}

// ── Arg parsing ──────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const getArg = (name) => {
  const i = args.indexOf(`--${name}`);
  return i !== -1 ? (args[i + 1] ?? null) : null;
};

const connectorsRaw = getArg("connectors") ?? "";
const captureDir = getArg("capture-dir");
const specDirArg = getArg("spec-dir");

if (!connectorsRaw || !captureDir || !specDirArg) {
  process.stderr.write(
    "Usage: find-specs-without-cassettes.mjs " +
      "--connectors <space-sep list> " +
      "--capture-dir <dir> " +
      "--spec-dir <dir>\n"
  );
  process.exit(1);
}

const connectors = connectorsRaw.split(/\s+/).filter(Boolean);

// Resolve spec dir relative to repo root (cypress-tests/)
const repoRoot = join(__dirname, "..");
const specDir = join(repoRoot, specDirArg);

// ── Find spec files ──────────────────────────────────────────────────────────

const specFiles = readdirSync(specDir)
  .filter((f) => f.endsWith(".cy.js"))
  .sort()
  .map((f) => join(specDir, f));

// The spec path used in the hash is relative to the cypress-tests/ root.
function specRelPath(absPath) {
  return relative(repoRoot, absPath).replace(/\\/g, "/");
}

// ── Check each spec across all connectors ────────────────────────────────────

const uncasseted = [];

for (const absSpec of specFiles) {
  const relSpec = specRelPath(absSpec);
  let content;
  try {
    content = readFileSync(absSpec, "utf8");
  } catch {
    continue;
  }

  const titles = extractTitlePaths(content);
  if (titles.length === 0) continue; // no tests found — skip

  // A spec is "uncasseted" for this batch if NO connector has cassettes for it.
  const anyConnectorHasCassettes = connectors.some((connector) =>
    titles.some((title) => {
      const hash = stableHashHex(`${connector}:${relSpec}:${title}`);
      return hasAnyCassette(hash, captureDir);
    })
  );

  if (!anyConnectorHasCassettes) {
    uncasseted.push(absSpec);
  }
}

if (uncasseted.length > 0) {
  process.stdout.write(uncasseted.join(" ") + "\n");
}
