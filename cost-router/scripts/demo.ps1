# Windows PowerShell entry point for scripts/demo.sh (same as `npm run demo`). Run up.ps1 first.
# Run: powershell -ExecutionPolicy Bypass -File cost-router\scripts\demo.ps1
& node (Join-Path $PSScriptRoot 'run-bash.mjs') 'scripts/demo.sh' @args
exit $LASTEXITCODE
