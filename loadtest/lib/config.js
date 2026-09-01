"use strict";

const fs = require("fs");
const path = require("path");

function parseScalar(value) {
  const trimmed = value.trim();
  if (trimmed === "") return "";
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null" || trimmed === "~") return null;
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) return Number(trimmed);
  if ((trimmed.startsWith('"') && trimmed.endsWith('"')) || (trimmed.startsWith("'") && trimmed.endsWith("'"))) return trimmed.slice(1, -1);
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
    const inner = trimmed.slice(1, -1).trim();
    return inner ? inner.split(",").map((item) => parseScalar(item)) : [];
  }
  return trimmed;
}

function parseYaml(source) {
  const root = {};
  const stack = [{ indent: -1, value: root }];
  for (const rawLine of source.split(/\r?\n/)) {
    if (!rawLine.trim() || rawLine.trimStart().startsWith("#")) continue;
    const indent = rawLine.match(/^ */)[0].length;
    const line = rawLine.trim();
    while (stack.length > 1 && indent <= stack[stack.length - 1].indent) stack.pop();
    const match = line.match(/^([^:]+):(.*)$/);
    if (!match) throw new Error(`Unsupported YAML line: ${rawLine}`);
    const parent = stack[stack.length - 1].value;
    const key = match[1].trim();
    if (match[2].trim() === "") {
      parent[key] = {};
      stack.push({ indent, value: parent[key] });
    } else {
      parent[key] = parseScalar(match[2]);
    }
  }
  return root;
}

function loadYaml(file) {
  if (!fs.existsSync(file)) throw new Error(`Config file not found: ${file}`);
  return parseYaml(fs.readFileSync(file, "utf8"));
}

function resolveMaybe(base, value) {
  if (!value) return value;
  return path.isAbsolute(String(value)) ? String(value) : path.resolve(base, String(value));
}

module.exports = { loadYaml, parseYaml, resolveMaybe };
