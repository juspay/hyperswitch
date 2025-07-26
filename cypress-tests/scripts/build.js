#!/usr/bin/env node
/* eslint-disable no-console */

import fs from "fs";

const DASHBOARD_DIR = "dashboard";
const DIST_DIR = "dist";

// Create dist directory
function createDirs() {
  if (!fs.existsSync(DIST_DIR)) {
    fs.mkdirSync(DIST_DIR, { recursive: true });
  }
}

// Minify JavaScript
function minifyJS(inputPath, outputPath) {
  let content = fs.readFileSync(inputPath, "utf8");

  // Preserve eslint disable comments
  const eslintComments = [];
  content = content.replace(/\/\*\s*eslint-disable[\s\S]*?\*\//g, (match) => {
    eslintComments.push(match);
    return `/*ESLINT_PRESERVE_${eslintComments.length - 1}*/`;
  });
  content = content.replace(/\/\*\s*global[\s\S]*?\*\//g, (match) => {
    eslintComments.push(match);
    return `/*ESLINT_PRESERVE_${eslintComments.length - 1}*/`;
  });

  // Basic minification
  content = content
    .replace(/\/\*[\s\S]*?\*\//g, "") // Remove other block comments
    .replace(/\/\/.*$/gm, "") // Remove line comments
    .replace(/\s+/g, " ") // Replace multiple spaces with single space
    .replace(/;\s*}/g, ";}") // Remove space before closing brace
    .replace(/{\s*/g, "{") // Remove space after opening brace
    .replace(/}\s*/g, "}") // Remove space after closing brace
    .trim();

  // Restore eslint comments
  eslintComments.forEach((comment, index) => {
    content = content.replace(`/*ESLINT_PRESERVE_${index}*/`, comment);
  });

  fs.writeFileSync(outputPath, content);
}

// Minify CSS
function minifyCSS(inputPath, outputPath) {
  let content = fs.readFileSync(inputPath, "utf8");

  // Basic CSS minification
  content = content
    .replace(/\/\*[\s\S]*?\*\//g, "") // Remove comments
    .replace(/\s+/g, " ") // Replace multiple spaces
    .replace(/;\s*}/g, ";}") // Remove space before closing brace
    .replace(/{\s*/g, "{") // Remove space after opening brace
    .replace(/}\s*/g, "}") // Remove space after closing brace
    .replace(/:\s*/g, ":") // Remove space after colon
    .replace(/;\s*/g, ";") // Remove space after semicolon
    .trim();

  fs.writeFileSync(outputPath, content);
}

// Process HTML with minified references
function processHTML() {
  let content = fs.readFileSync(`${DASHBOARD_DIR}/index.html`, "utf8");

  // Update references to minified files
  content = content
    .replace("dashboard.js", "dashboard.min.js")
    .replace("styles-minimal.css", "styles-minimal.min.css");

  fs.writeFileSync(`${DIST_DIR}/index.html`, content);
}

// Main build function
function build() {
  console.log("Building dashboard...");

  createDirs();

  // Minify files
  minifyJS(`${DASHBOARD_DIR}/dashboard.js`, `${DIST_DIR}/dashboard.min.js`);
  minifyCSS(
    `${DASHBOARD_DIR}/styles-minimal.css`,
    `${DIST_DIR}/styles-minimal.min.css`
  );

  // Process HTML
  processHTML();

  // Copy vercel.json if it exists
  if (fs.existsSync(`${DASHBOARD_DIR}/vercel.json`)) {
    fs.copyFileSync(`${DASHBOARD_DIR}/vercel.json`, `${DIST_DIR}/vercel.json`);
  }

  console.log("✅ Build completed! Files ready in ./dist directory");
}

// Main script logic
build();
