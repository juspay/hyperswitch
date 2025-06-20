import CleanCSS from "clean-css";
import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { minify as terserMinify } from "terser";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const dashboardDir = join(__dirname, "../dashboard");
const buildDir = join(dashboardDir, "build");
const sourceJsPath = join(dashboardDir, "dashboard.js");
const sourceCssPath = join(dashboardDir, "styles-minimal.css");
const sourceHtmlPath = join(dashboardDir, "index.html");

const outputJsPath = join(buildDir, "dashboard.min.js");
const outputCssPath = join(buildDir, "styles-minimal.min.css");
const outputHtmlPath = join(buildDir, "index.html");

async function buildDashboard() {
  try {
    // 1. Create build directory
    mkdirSync(buildDir, { recursive: true });
    console.log(`Created build directory: ${buildDir}`);

    // 2. Minify JavaScript
    const jsCode = readFileSync(sourceJsPath, "utf8");
    const terserResult = await terserMinify(jsCode, {
      mangle: {
        toplevel: true,
      },
      compress: {
        drop_console: true,
      },
    });
    if (terserResult.error) {
      throw terserResult.error;
    }
    writeFileSync(outputJsPath, terserResult.code);
    console.log(`Minified JavaScript to: ${outputJsPath}`);

    // 3. Minify CSS
    const cssCode = readFileSync(sourceCssPath, "utf8");
    const cleanCss = new CleanCSS({});
    const cssResult = cleanCss.minify(cssCode);
    if (cssResult.errors && cssResult.errors.length > 0) {
      throw new Error(
        `CSS Minification Errors: ${cssResult.errors.join(", ")}`
      );
    }
    writeFileSync(outputCssPath, cssResult.styles);
    console.log(`Minified CSS to: ${outputCssPath}`);

    // 4. Copy and update HTML
    let htmlContent = readFileSync(sourceHtmlPath, "utf8");
    htmlContent = htmlContent.replace("dashboard.js", "dashboard.min.js");
    htmlContent = htmlContent.replace(
      "styles-minimal.css",
      "styles-minimal.min.css"
    );
    writeFileSync(outputHtmlPath, htmlContent);
    console.log(`Copied and updated HTML to: ${outputHtmlPath}`);

    console.log("Dashboard build complete!");
  } catch (error) {
    console.error("Error during dashboard build:", error);
    process.exit(1);
  }
}

buildDashboard();
