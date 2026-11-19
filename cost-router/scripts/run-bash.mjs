// Runs one of the bash scripts with a real bash on every OS: `node scripts/run-bash.mjs scripts/up.sh [args...]`.
// Used by `npm run up|down|demo` and the PowerShell wrappers. On Windows a plain `bash` can resolve to WSL's
// C:\Windows\System32\bash.exe, so Git Bash is located explicitly (set GIT_BASH to override).
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const [script, ...args] = process.argv.slice(2);
if (!script) {
  console.error('usage: node scripts/run-bash.mjs <script.sh> [args...]');
  process.exit(2);
}

function findGitBash() {
  const candidates = [
    process.env.GIT_BASH,
    'C:\\Program Files\\Git\\bin\\bash.exe',
    'C:\\Program Files (x86)\\Git\\bin\\bash.exe',
    process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, 'Programs', 'Git', 'bin', 'bash.exe'),
  ];
  try {
    // git.exe lives in <Git>\cmd\ (or similar); bash.exe is in <Git>\bin\.
    for (const git of execFileSync('where', ['git'], { encoding: 'utf8' }).split(/\r?\n/).filter(Boolean)) {
      candidates.push(resolve(dirname(git), '..', 'bin', 'bash.exe'), resolve(dirname(git), '..', '..', 'bin', 'bash.exe'));
    }
  } catch {
    /* git not on PATH */
  }
  return candidates.find((c) => c && existsSync(c));
}

const bash = process.platform === 'win32' ? findGitBash() : 'bash';
if (!bash) {
  console.error('Git Bash not found. Install Git for Windows (https://git-scm.com) or set GIT_BASH to its bin\\bash.exe.');
  process.exit(1);
}
const target = resolve(ROOT, script).replace(/\\/g, '/');
const result = spawnSync(bash, [target, ...args], { stdio: 'inherit', cwd: ROOT });
if (result.error) {
  console.error(`could not run ${bash}: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
