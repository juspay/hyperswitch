# Windows PowerShell entry point for scripts/down.sh (same as `npm run down`). Add -v to also delete the database volume.
# Run: powershell -ExecutionPolicy Bypass -File cost-router\scripts\down.ps1
& node (Join-Path $PSScriptRoot 'run-bash.mjs') 'scripts/down.sh' @args
exit $LASTEXITCODE
