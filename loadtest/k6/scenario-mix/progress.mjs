import { clearLine, cursorTo } from 'node:readline';

// Redraw one terminal line; keep redirected output readable and bounded.
export function progress(label, total) {
  let lastTime = 0, lastBucket = -1;
  const terminal = Boolean(process.stdout.isTTY);
  function update(done, detail = '') {
    const fraction = total > 0 ? Math.min(1, done / total) : 1;
    const bucket = Math.floor(fraction * 10);
    const now = Date.now();
    if (done !== total && (terminal ? now - lastTime < 200 : bucket === lastBucket)) return;
    lastTime = now; lastBucket = bucket;
    const filled = Math.floor(fraction * 20);
    const line = `${label} [${'='.repeat(filled)}${'-'.repeat(20 - filled)}] ${Math.floor(fraction * 100)}% ${done}/${total}${detail ? ` | ${detail}` : ''}`;
    if (terminal) { clearLine(process.stdout, 0); cursorTo(process.stdout, 0); process.stdout.write(line); }
    else console.log(line);
  }
  update(0);
  return { update, finish() { if (terminal) process.stdout.write('\n'); } };
}
