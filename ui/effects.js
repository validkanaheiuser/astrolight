'use strict';
// Lighting maths for the on-screen preview. Every formula mirrors src-tauri/src/effects.rs;
// scripts/check-effects.cjs compares the two channel by channel.

// id, short tag, colour A, colour B, category, added in 3.3
const EFFECTS = [
  ['paint', 'Per-key', '#b7a1ff', '#66d9d0', 'static'],
  ['solid', 'Solid', '#aebfff', '#aebfff', 'static'],
  ['gradient', 'Gradient', '#66d9d0', '#a18fff', 'static'],
  ['wave', 'Wave', '#38d9bb', '#a77aff', 'calm'],
  ['breathe', 'Breathe', '#8aa9ff', '#ca90ff', 'calm'],
  ['rainbow', 'Rainbow', '#f488bb', '#75d8e6', 'energy'],
  ['aurora', 'Aurora', '#48e8be', '#935bff', 'calm'],
  ['spiral', 'Spiral', '#6eeaff', '#e79aff', 'geometry'],
  ['ripple', 'Ripple', '#427efc', '#b1f8ff', 'geometry'],
  ['meteor', 'Meteor', '#e6c4ff', '#755bdf', 'energy'],
  ['stars', 'Starlight', '#ffd990', '#c6b5ff', 'calm'],
  ['rain', 'Rain', '#4ce0bb', '#91aaff', 'calm'],
  ['fire', 'Fire', '#ffcf58', '#f04f45', 'energy'],
  ['checker', 'Interference', '#f68dbb', '#678dff', 'geometry'],
  ['beacon', 'Beacon', '#6ce1de', '#4e62dd', 'geometry'],
  ['sunset', 'Sunset', '#ffad66', '#b058cc', 'calm'],
  ['plasma', 'Plasma', '#ff68ba', '#7d77ff', 'energy'],
  ['nebula', 'Nebula', '#7f69ff', '#f2a1db', 'calm'],
  ['ocean', 'Ocean', '#167de2', '#70ead8', 'calm'],
  ['comets', 'Comets', '#d1b3ff', '#62e8ed', 'energy'],
  ['fireflies', 'Fireflies', '#d7f77d', '#60cda5', 'calm'],
  ['chevron', 'Chevron', '#ffb072', '#ba79ff', 'geometry'],
  ['diamonds', 'Diamonds', '#79deef', '#e6afff', 'geometry'],
  ['helix', 'Double helix', '#ff87b6', '#73d5fa', 'geometry'],
  ['ribbon', 'Silk ribbon', '#d1a5ff', '#8ae4dc', 'calm'],
  ['scanline', 'Scanner', '#f583a3', '#5ecbe8', 'energy'],
  ['heartbeat', 'Heartbeat', '#ef5478', '#ffb19a', 'energy'],
  ['pinwheel', 'Pinwheel', '#e994f3', '#7edbe5', 'geometry'],
  ['cycle', 'Spectrum', '#ff6b9d', '#6bd5ff', 'calm', 1],
  ['prism', 'Prism', '#8f7bff', '#5fe3d0', 'geometry', 1],
  ['vortex', 'Vortex', '#7c5cff', '#ff9ad5', 'geometry', 1],
  ['kaleido', 'Kaleidoscope', '#ffd36b', '#ff6bb5', 'geometry', 1],
  ['bloom', 'Bloom', '#ff8fc7', '#8a7dff', 'calm', 1],
  ['matrix', 'Digital rain', '#29e06c', '#d8ffe4', 'energy', 1],
  ['lightning', 'Lightning', '#5b6cff', '#f4f1ff', 'energy', 1],
  ['bounce', 'Bouncing ball', '#ff9f5a', '#ff5a8a', 'energy', 1],
  ['marquee', 'Marquee', '#ffd166', '#ff7a59', 'geometry', 1],
  ['mosaic', 'Mosaic', '#5ad1e6', '#b58cff', 'calm', 1],
  ['swirl', 'Swirl', '#ff7eb6', '#6ee7d8', 'calm', 1],
  ['zigzag', 'Zigzag', '#9bf06b', '#3fc8e0', 'geometry', 1],
  ['lava', 'Lava lamp', '#ff5a3c', '#ffc24a', 'calm', 1],
  ['candle', 'Candlelight', '#ffb347', '#ff6a2b', 'calm', 1],
  ['police', 'Strobe', '#ff3b4e', '#3b6bff', 'energy', 1],
  ['bars', 'Equalizer', '#4be3a0', '#e85cff', 'energy', 1],
  ['snake', 'Snake', '#7df27a', '#2fb7c9', 'geometry', 1],
  ['sparkle', 'Sparkle', '#5a63c9', '#ffffff', 'calm', 1],
  ['rpulse', 'Key pulse', '#ffd08a', '#7a6bff', 'reactive', 1],
  ['rglow', 'Key glow', '#7df0ff', '#5a55d6', 'reactive', 1],
  ['rripple', 'Key ripple', '#b9a4ff', '#4fd8c4', 'reactive', 1],
  ['rcross', 'Key cross', '#ff8fb8', '#5b7bff', 'reactive', 1],
  ['rheat', 'Heatmap', '#ff5a3c', '#3b6bff', 'reactive', 1],
  ['rrainbow', 'Rainbow splash', '#ffffff', '#6b63c9', 'reactive', 1],
];
const EFFECT_BY_ID = Object.fromEntries(EFFECTS.map(e => [e[0], e]));

const Lights = (() => {
  const TAU = Math.PI * 2;
  const STATIC = new Set(['paint', 'solid', 'gradient']);
  const REACTIVE = new Set(['rpulse', 'rglow', 'rripple', 'rcross', 'rheat', 'rrainbow']);
  const fract = x => x - Math.floor(x);
  const mod = (x, n) => ((x % n) + n) % n;
  const clamp = (x, lo = 0, hi = 1) => Math.max(lo, Math.min(hi, x));
  const hash = n => fract(Math.sin(n * 12.9898 + 4.1414) * 43758.5453);
  const tri = x => Math.abs(fract(x) * 2 - 1);
  const smooth = (lo, hi, x) => { const t = clamp((x - lo) / (hi - lo)); return t * t * (3 - 2 * t); };
  const rgb = s => [1, 3, 5].map(i => parseInt(s.slice(i, i + 2), 16));
  const hueRgb = hue => [0, 8, 4].map(o => {
    const k = mod(o + mod(hue, 1) * 12, 12);
    return 255 * (1 - Math.max(0, Math.min(k - 3, 9 - k, 1)));
  });
  const reactRate = speed => 0.4 + speed / 40;
  const overridesOf = c => (c.perKey || []);

  // Per-key geometry only depends on the layout and angle, so it is cached between frames.
  let geoKey = '', geoFor = null, geo = [];
  function geometry(keys, layout, angleDeg) {
    const id = layout + ':' + angleDeg;
    if (geoFor === keys && geoKey === id) return geo;
    const angle = angleDeg * Math.PI / 180, sn = Math.sin(angle), cs = Math.cos(angle);
    geo = keys.map((key, i) => {
      const box = key.layouts[layout], x = (box.x + box.w / 2) / 760, y = (box.y + box.h / 2) / 270;
      const u = (x - .5) * cs + (y - .5) * sn + .5;
      return {
        u, v: -(x - .5) * sn + (y - .5) * cs + .5,
        r: Math.hypot(x - .5, (y - .5) * .6),
        ang: Math.atan2((y - .5) * .6, x - .5) / TAU,
        seed: fract(Math.sin(i * 12.9898) * 43758.5453),
        rainSeed: fract(Math.sin(Math.floor(u * 15) * 12.9898) * 43758.5453),
        gx: x, gy: y * 270 / 760,
      };
    });
    geoFor = keys; geoKey = id;
    return geo;
  }

  function react(mode, a, b, scale, i, clock, rate, hits) {
    const g = geo[i];
    let level = 0;
    const color = [0, 0, 0];
    for (const [key, at] of hits) {
      const age = (clock - at) * rate;
      if (age < 0 || key >= geo.length) continue;
      const h = geo[key], dx = g.gx - h.gx, dy = g.gy - h.gy, d = Math.sqrt(dx * dx + dy * dy);
      switch (mode) {
        case 'rpulse': if (key === i) level += Math.exp(-age * 2.2); break;
        case 'rglow': { const sigma = .05 + scale * .03; level += Math.exp(-age * 2) * Math.exp(-d * d / (2 * sigma * sigma)); break; }
        case 'rripple': level += Math.exp(-Math.pow((d - age * .55) / .05, 2)) * Math.exp(-age * 1.3); break;
        case 'rcross': {
          const along = Math.abs(dy) < .02 ? Math.abs(dx) : Math.abs(dx) < .035 ? Math.abs(dy) * 3 : null;
          if (along !== null) level += Math.exp(-Math.pow((along - age * .9) / .07, 2)) * Math.exp(-age * 1.4);
          break;
        }
        case 'rheat': level += .45 * Math.exp(-age * .35) * Math.exp(-d * d / .0098); break;
        default: {
          const w = Math.exp(-Math.pow((d - age * .55) / .06, 2)) * Math.exp(-age * 1.2);
          const tint = hueRgb(fract(at * .37 + key * .113));
          for (let j = 0; j < 3; j++) color[j] += tint[j] * w;
          level += w;
        }
      }
    }
    level = Math.min(level, 1);
    if (mode === 'rheat') return [0, 1, 2].map(j => (b[j] * (1 - level) + a[j] * level) * (.12 + .88 * level));
    if (mode === 'rrainbow') return [0, 1, 2].map(j => Math.min(b[j] * .1 * (1 - level) + color[j], 255));
    return [0, 1, 2].map(j => b[j] * .12 * (1 - level) + a[j] * level);
  }

  function shade(mode, a, b, scale, i, phase, clock, rate, hits, paint) {
    const g = geo[i], u = g.u, v = g.v, r = g.r, ang = g.ang;
    let t = 0, gain = 1, hue = null;
    switch (mode) {
      case 'paint': return paint;
      case 'gradient': t = clamp(u); break;
      case 'wave': t = (Math.sin(u * scale * TAU - phase * 2) + 1) * .5; break;
      case 'breathe': gain = .06 + .94 * (Math.sin(phase * 2) + 1) * .5; break;
      case 'rainbow': hue = u * scale * .5 - phase * .12; break;
      case 'aurora': t = (Math.sin(u * 5 + Math.sin(v * 3 + phase) * 1.2 - phase) + 1) * .5; gain = .2 + .8 * (Math.sin(v * 4 - phase * 1.3 + u * 2) + 1) * .5; break;
      case 'spiral': hue = ang + r * scale - phase * .18; break;
      case 'ripple': t = Math.min(r, 1); gain = .08 + .92 * Math.pow(Math.sin(r * scale * 15 - phase * 4) * .5 + .5, 3); break;
      case 'meteor': { const head = mod(phase * .22, 1.5) - .25, d = head - u; gain = d >= 0 ? Math.exp(-d * (6 + scale * 3)) : Math.exp(-d * d * 800); t = v; break; }
      case 'stars': { const seed = g.seed; gain = .02 + .98 * Math.pow(Math.sin(phase * (.6 + seed) + seed * 25) * .5 + .5, 8 - scale); t = seed; break; }
      case 'rain': { const seed = g.rainSeed, head = mod(phase * (.3 + seed * .35) + seed * 3, 1.7) - .3, d = head - v; gain = d >= 0 ? Math.exp(-d * 8) : Math.exp(-d * d * 300); t = seed; break; }
      case 'fire': { const noise = Math.sin(u * 12 + phase * 2.5) * Math.cos(u * 7 - phase * 3.1); t = clamp(v * .6 + noise * .25 + .2); gain = clamp(.35 + v * .6 + noise * .2); break; }
      case 'checker': t = (Math.sin(u * scale * 10 + phase) * Math.sin(v * scale * 6 - phase) + 1) * .5; break;
      case 'beacon': t = (Math.cos(ang * TAU - phase * 2) + 1) * .5; gain = .08 + .92 * Math.pow(t, 6); break;
      case 'sunset': t = Math.sin((u + v * .3) * scale + phase * .15) * .5 + .5; break;
      case 'plasma': t = .5 + (Math.sin(u * scale * 7 + phase) + Math.sin(v * scale * 7 - phase * .8) + Math.sin(r * scale * 12 - phase * 1.2)) / 6; break;
      case 'nebula': { const cloud = Math.sin(u * scale * 3 + Math.sin(v * 5 - phase * .25) + phase * .35); t = cloud * .5 + .5; gain = .25 + .75 * (Math.sin(u * 4 - v * 3 + phase * .4) * .5 + .5); break; }
      case 'ocean': { const wave = Math.sin(u * scale * 5 - phase * 1.3 + Math.sin(v * 6 + phase * .6)); t = wave * .5 + .5; gain = .35 + .65 * (Math.sin(v * 8 - phase + u * 2) * .5 + .5); break; }
      case 'comets': { const d = mod(u + v * .28 - phase * .18, .48); gain = Math.exp(-d * (10 + scale * 4)); t = Math.sin(v * 3 + phase * .25) * .5 + .5; break; }
      case 'fireflies': { const seed = g.seed; t = seed; gain = .015 + .985 * Math.pow(Math.sin(phase * (.35 + seed * .3) + seed * 30) * .5 + .5, 12) * (Math.sin(u * 5 - phase * .4) * .3 + .7); break; }
      case 'chevron': t = Math.sin((u + Math.abs(v - .5) * .8) * scale * 8 - phase * 2) * .5 + .5; gain = .15 + .85 * t; break;
      case 'diamonds': { const d = Math.abs(u - .5) + Math.abs(v - .5) * .6; t = Math.sin(d * scale * 12 - phase * 2) * .5 + .5; gain = .1 + .9 * t * t; break; }
      case 'helix': { const center = Math.sin(u * scale * 6 - phase * 1.5) * .3, one = Math.exp(-Math.pow(v - .5 - center, 2) * 100), two = Math.exp(-Math.pow(v - .5 + center, 2) * 100); t = two / (one + two + 1e-9); gain = Math.min(one + two, 1); break; }
      case 'ribbon': { const center = .5 + Math.sin(u * scale * 3 - phase * .7) * .25; gain = .05 + .95 * Math.exp(-Math.pow(v - center, 2) * (12 + scale * 8)); t = Math.sin(u * 3 + phase * .4) * .5 + .5; break; }
      case 'scanline': { const head = Math.sin(phase * .9) * .5 + .5; gain = .02 + .98 * Math.exp(-Math.pow(u - head, 2) * (45 + scale * 30)); t = clamp(v); break; }
      case 'heartbeat': { const beat = mod(phase * .55, 1); gain = .04 + .96 * Math.min(Math.exp(-Math.pow(beat - .2, 2) * 220) + .65 * Math.exp(-Math.pow(beat - .42, 2) * 300), 1); t = Math.min(r, 1); break; }
      case 'pinwheel': hue = ang * (2 + scale) + phase * .15 + r * .6; break;
      case 'cycle': hue = phase * .1 + u * .04 * scale; break;
      case 'prism': hue = r * scale * .5 + phase * .14; break;
      case 'vortex': t = Math.sin(ang * TAU * 2 + r * scale * 14 - phase * 2) * .5 + .5; break;
      case 'kaleido': { const fold = Math.abs(fract(ang * 3) - .5) * 2; hue = fold * .5 + r * scale * .5 - phase * .1; break; }
      case 'bloom': t = Math.sin(r * scale * 8 - Math.sin(phase * .8) * 3) * .5 + .5; gain = .2 + .8 * (Math.sin(ang * TAU * 5 + Math.sin(phase * .7) * 2) * .5 + .5); break;
      case 'matrix': {
        const seed = hash(Math.floor(u * 19)), head = mod(phase * (.25 + seed * .4) + seed * 5, 1.6) - .2;
        const d = head - (Math.floor(v * 5) + .5) / 5, tip = Math.exp(-d * d * 900);
        t = tip; gain = Math.max(d >= 0 ? Math.exp(-d * 5) : 0, tip); break;
      }
      case 'lightning': {
        const n = Math.floor(phase * 1.4), f = fract(phase * 1.4);
        const flash = hash(n) > .45 ? Math.exp(-f * 7) * (.6 + .4 * Math.abs(Math.sin(f * 60))) : 0;
        const lean = (hash(n + .7) - .5) * .6;
        const bolt = Math.exp(-Math.pow(u - hash(n + .5) - (v - .5) * lean, 2) * (30 + scale * 20));
        t = bolt; gain = .04 + flash * (.25 + .75 * bolt); break;
      }
      case 'bounce': { const dx = u - tri(phase * .23), dy = (v - tri(phase * .31 + .3)) * .36; gain = .05 + .95 * Math.exp(-(dx * dx + dy * dy) * (40 + scale * 40)); t = tri(phase * .05); break; }
      case 'marquee': { const s = Math.sin((u * scale * 3 - phase * .35) * TAU); gain = .06 + .94 * clamp(s * 4 + .5); t = clamp(v); break; }
      case 'mosaic': {
        const z = phase * .5 + g.seed * 7, n = Math.floor(z), f = smooth(0, 1, fract(z));
        t = hash(i * 3.1 + n) * (1 - f) + hash(i * 3.1 + n + 1) * f;
        gain = .3 + .7 * (hash(i * 1.7 + n) * (1 - f) + hash(i * 1.7 + n + 1) * f); break;
      }
      case 'swirl': {
        const sn = Math.sin(phase * .9), cs = Math.cos(phase * .9), k = 18 + scale * 10;
        const one = Math.exp(-(Math.pow(u - .5 - .32 * cs, 2) + Math.pow((v - .5 - .3 * sn) * .45, 2)) * k);
        const two = Math.exp(-(Math.pow(u - .5 + .32 * cs, 2) + Math.pow((v - .5 + .3 * sn) * .45, 2)) * k);
        t = two / (one + two + 1e-9); gain = Math.min(.04 + one + two, 1); break;
      }
      case 'zigzag': { const center = .15 + .7 * tri(u * scale * 1.5 - phase * .2); gain = .06 + .94 * Math.exp(-Math.pow(v - center, 2) * 40); t = clamp(u); break; }
      case 'lava': {
        const n = Math.sin(u * scale * 2.3 + phase * .4) + Math.sin(v * 3.1 - phase * .33) + Math.sin((u + v) * scale * 1.7 + phase * .27) + Math.sin(r * 6 - phase * .2);
        const m = n / 8 + .5; t = smooth(.35, .65, m); gain = .2 + .8 * smooth(.2, .8, m); break;
      }
      case 'candle': {
        const seed = g.seed, flicker = .55 + .25 * Math.sin(phase * 5.3) + .12 * Math.sin(phase * 12.7 + seed * 6) + .08 * Math.sin(phase * 23.1 + seed * 40);
        gain = clamp(flicker * (.6 + .4 * v)); t = clamp(1 - v * .8 + .1 * Math.sin(phase * 3 + seed * 10)); break;
      }
      case 'police': {
        const beat = fract(phase * .6), strobe = clamp(Math.sin(beat * TAU * 6) * 3 + .5), left = u < .5;
        gain = left === (beat < .5) ? .15 + .85 * strobe : .03; t = left ? 0 : 1; break;
      }
      case 'bars': {
        const seed = hash(Math.floor(u * 14));
        const level = .15 + .85 * (.5 + .5 * Math.sin(phase * (1.2 + seed * 2.2) + seed * 20)) * (.6 + .4 * Math.sin(phase * .7 + seed * 9));
        const height = 1 - v; gain = .04 + .96 * clamp((level - height) * 8 + .5); t = clamp(height); break;
      }
      case 'snake': {
        const row = clamp(Math.floor(v * 5), 0, 4), along = row % 2 === 0 ? u : 1 - u;
        const d = fract(fract(phase * .12) - (row + clamp(along)) / 5);
        gain = .03 + .97 * Math.exp(-d * (6 + scale * 4)); t = Math.min(d * 3, 1); break;
      }
      case 'sparkle': { const z = phase * 3 + g.seed * 10, glint = hash(i * 7.3 + Math.floor(z)) > .82 ? Math.sin(fract(z) * Math.PI) : 0; t = glint; gain = .25 + .75 * glint; break; }
      case 'rpulse': case 'rglow': case 'rripple': case 'rcross': case 'rheat': case 'rrainbow':
        return react(mode, a, b, scale, i, clock, rate, hits);
    }
    if (hue !== null) return hueRgb(hue);
    return [0, 1, 2].map(j => (a[j] * (1 - t) + b[j] * t) * gain);
  }

  return {
    rgb,
    isStatic: mode => STATIC.has(mode),
    isReactive: mode => REACTIVE.has(mode),
    /** True when nothing in the configuration moves on its own or reacts to presses. */
    static: c => STATIC.has(c.mode) && overridesOf(c).every(o => !o || STATIC.has(o.mode)),
    reactive: c => REACTIVE.has(c.mode) || overridesOf(c).some(o => o && REACTIVE.has(o.mode)),
    /** Colours for every key as [r, g, b]. `hits` is a list of [keyIndex, clockAtPress]. */
    frame(c, keys, phase, clock = phase, hits = []) {
      geometry(keys, c.layout, c.angle || 0);
      const a = rgb(c.colorA), b = rgb(c.colorB), brightness = c.brightness / 100;
      const scale = .5 + (c.scale || 50) / 33.3333333333, perKey = overridesOf(c);
      const out = new Array(keys.length);
      for (let i = 0; i < keys.length; i++) {
        const o = perKey[i];
        const value = o
          ? shade(o.mode, rgb(o.colorA), rgb(o.colorB), scale, i, clock * o.speed / 50, clock, reactRate(o.speed), hits, null)
          : shade(c.mode, a, b, scale, i, phase, clock, reactRate(c.speed), hits, c.mode === 'paint' ? rgb(c.colors[i]) : null);
        out[i] = [clamp(Math.round(value[0] * brightness), 0, 255), clamp(Math.round(value[1] * brightness), 0, 255), clamp(Math.round(value[2] * brightness), 0, 255)];
      }
      return out;
    },
  };
})();
