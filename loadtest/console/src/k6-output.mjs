export function k6Output(output = '') {
  const lines = output.replace(/\x1b\[[0-9;?]*[A-Za-z]/g, '').replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n');
  const history = [];
  let frame = [], inFrame = false, framePosition = 0;
  for (const line of lines) {
    if (/^\s*running \(/.test(line)) {
      frame = [line.trimEnd()];
      framePosition = history.length;
      inFrame = true;
    } else if (/^\s*\S+\s+\[\s*\d+%\s*\].*(?:VUs|iters\/s)/.test(line)) {
      // A progress row may survive truncation of the stored output buffer.
      const name = line.trim().split(/\s/)[0];
      const index = frame.findIndex(row => row.trim().startsWith(name + ' '));
      if (index < 0) frame.push(line.trimEnd()); else frame[index] = line.trimEnd();
      inFrame = true;
    } else if (line.trim()) {
      history.push(line);
      inFrame = false;
    } else if (!inFrame && history.length && history.at(-1) !== '') {
      history.push('');
    }
  }
  return {progress: frame.join('\n'), history: history.join('\n').trim(),
    before: history.slice(0,framePosition).join('\n').trimEnd(),
    after: history.slice(framePosition).join('\n').trim()};
}
