# Windows PowerShell entry point for scripts/up.sh (same as `npm run up`). Git Bash is located automatically.
# Run: powershell -ExecutionPolicy Bypass -File cost-router\scripts\up.ps1
& node (Join-Path $PSScriptRoot 'run-bash.mjs') 'scripts/up.sh' @args
exit $LASTEXITCODE
