export function formatDuration(seconds) {
  if(!Number.isFinite(Number(seconds))||Number(seconds)<0)return '—';
  const total=Math.round(Number(seconds)),hours=Math.floor(total/3600),minutes=Math.floor(total%3600/60),remaining=total%60;
  return [hours?`${hours} hr`:'',minutes?`${minutes} min`:'',remaining||!total?`${remaining} sec`:''].filter(Boolean).join(' ');
}
