// The sound for a find. It is made here from plain tones, so the app ships no sound file.
export type PingSound = 'soft' | 'loud';

let ctx: AudioContext | null = null;

/** One note: a quick rise, then it rings out. */
function note(c: AudioContext, at: number, freq: number, len: number, gain: number, type: OscillatorType) {
  const osc = c.createOscillator();
  const vol = c.createGain();
  osc.type = type;
  osc.frequency.value = freq;
  vol.gain.setValueAtTime(0.0001, at);
  vol.gain.exponentialRampToValueAtTime(gain, at + 0.008);
  vol.gain.exponentialRampToValueAtTime(0.0001, at + len);
  osc.connect(vol).connect(c.destination);
  osc.start(at);
  osc.stop(at + len + 0.05);
}

/** Soft is two quiet bell notes. Loud is a bright three-note chime, played twice. */
export function ping(sound: PingSound) {
  try {
    ctx ??= new AudioContext();
    if (ctx.state === 'suspended') void ctx.resume();
    const t = ctx.currentTime + 0.02;
    if (sound === 'soft') {
      note(ctx, t, 880, 0.7, 0.09, 'sine');
      note(ctx, t + 0.14, 1174.66, 0.9, 0.09, 'sine');
    } else {
      for (const again of [0, 0.5]) {
        [880, 1108.73, 1318.51].forEach((f, i) => {
          note(ctx!, t + again + i * 0.11, f, 0.5, 0.45, 'triangle');
          note(ctx!, t + again + i * 0.11, f * 2, 0.3, 0.14, 'sine');
        });
      }
    }
  } catch {
    // no sound device: the hunt carries on in silence
  }
}
