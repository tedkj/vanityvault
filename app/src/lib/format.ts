export function big(n: number): string {
  if (!isFinite(n)) return 'too many';
  if (n < 1e3) return String(Math.round(n));
  const units: [number, string][] = [
    [1e15, 'quadrillion'],
    [1e12, 'trillion'],
    [1e9, 'billion'],
    [1e6, 'million'],
    [1e3, 'thousand'],
  ];
  for (const [v, name] of units) {
    if (n >= v) {
      const x = n / v;
      return `${x >= 100 ? Math.round(x) : x.toFixed(x >= 10 ? 1 : 2)} ${name}`;
    }
  }
  return String(n);
}

export function wait(sec: number): string {
  if (!isFinite(sec) || sec <= 0) return 'unknown';
  const plural = (n: number, w: string) => `about ${n} ${w}${n === 1 ? '' : 's'}`;
  if (sec < 1.5) return 'about a second';
  if (sec < 90) return plural(Math.round(sec), 'second');
  if (sec < 5400) return plural(Math.round(sec / 60), 'minute');
  if (sec < 129600) return plural(Math.round(sec / 3600), 'hour');
  if (sec < 86400 * 60) return plural(Math.round(sec / 86400), 'day');
  if (sec < 86400 * 365 * 2) return plural(Math.round(sec / (86400 * 30.4)), 'month');
  const y = sec / (86400 * 365);
  return y > 1e6 ? 'longer than anyone can wait' : plural(Math.round(y), 'year');
}

export function clock(ms: number): string {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const pad = (n: number) => String(n).padStart(2, '0');
  return h ? `${h}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}
