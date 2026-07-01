import http from "http";
import https from "https";

export function registerVaultHealthTask(on) {
  on("task", {
    checkVaultHealth: ({ vaultUrl }) => {
      return new Promise((resolve) => {
        const httpModule = vaultUrl.startsWith("https") ? https : http;
        const url = new URL(vaultUrl + "/health");
        const options = {
          hostname: url.hostname,
          port: url.port || (vaultUrl.startsWith("https") ? 443 : 80),
          path: url.pathname,
          method: "GET",
          timeout: 3000,
        };
        const req = httpModule.request(options, (res) => {
          resolve({ status: res.statusCode, healthy: res.statusCode === 200 });
        });
        req.on("error", () => resolve({ status: 0, healthy: false }));
        req.on("timeout", () => {
          req.destroy();
          resolve({ status: 0, healthy: false });
        });
        req.end();
      });
    },
  });
}
