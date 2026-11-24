'use strict';
/* Astrolight interface. Sections: helpers · i18n · status · lighting · keyboard · preview ·
   presets · remap · macros · settings and profiles · boot. */

const $ = id => document.getElementById(id);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const clone = x => structuredClone(x);
const invoke = (name, args = {}) => window.__TAURI__.core.invoke(name, args);
const put = (id, text) => { const el = $(id); if (el.textContent !== text) el.textContent = text; };
const el = (tag, className = '', text = '') => { const n = document.createElement(tag); if (className) n.className = className; if (text) n.textContent = text; return n; };
const button = (text, fn, className = '') => { const b = el('button', className, text); b.onclick = fn; return b; };
const option = (value, label) => { const o = el('option', '', label); o.value = value; return o; };
const fold = s => s.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/đ/g, 'd').toLowerCase();
const pressed = (b, on) => { b.classList.toggle('active', on); b.setAttribute('aria-pressed', String(on)); };

let keys = [], codes = [], config, settings, map, defaultMap, presets = [];
let status = { connected: false, running: false, activeProfile: '' };
let page = 'lighting', scope = 'board', busy = false, nativeMinimized = false, lang = 'en';

/* ── i18n ─────────────────────────────────────────────────────────────── */
function t(key, vars) {
  let text = I18N[lang]?.[key] ?? I18N.en[key] ?? key;
  if (vars) for (const name in vars) text = text.replaceAll('{' + name + '}', vars[name]);
  return text;
}
const has = key => key in I18N.en || key in (I18N[lang] || {});
const fxName = id => t('fx.' + id);
function errText(error) {
  const raw = String(error?.message ?? error);
  if (!raw.startsWith('E:')) return raw;
  const [, code, ...rest] = raw.split(':'), detail = rest.join(':');
  return has('err.' + code) ? t('err.' + code, { detail: detail.startsWith('E:') ? errText(detail) : detail }) : raw;
}
function applyI18n() {
  document.documentElement.lang = lang === 'zh' ? 'zh-Hans' : lang;
  for (const n of $$('[data-i18n]')) n.textContent = t(n.dataset.i18n);
  for (const n of $$('[data-i18n-attr]')) for (const pair of n.dataset.i18nAttr.split(';')) { const [attr, key] = pair.split(':'); n.setAttribute(attr, t(key)); }
}
function setLanguage(next, save = true) {
  lang = I18N[next] ? next : 'en';
  $('language').value = $('language-quick').value = lang;
  applyI18n();
  if (!config) return;
  renderAll();
  if (save && settings.language !== lang) { settings.language = lang; saveOptions(); }
}
for (const id of ['language', 'language-quick']) {
  for (const [code, name] of LANGUAGES) $(id).append(option(code, name));
  $(id).onchange = e => setLanguage(e.target.value);
}

/* ── toasts, dialogs, busy buttons ────────────────────────────────────── */
function toast(text, type = '') {
  const box = $('toasts');
  if (box.lastChild?.textContent === text) box.lastChild.remove();
  while (box.children.length > 2) box.firstChild.remove();
  const item = el('div', 'toast ' + type, text);
  box.append(item);
  setTimeout(() => { item.classList.add('leaving'); setTimeout(() => item.remove(), 220); }, type === 'error' ? 8000 : 3600);
}
const fail = error => toast(errText(error), 'error');
function action(id, fn) {
  $(id).onclick = async () => {
    if (busy) return;
    busy = true;
    const b = $(id);
    b.disabled = true; b.setAttribute('aria-busy', 'true');
    try { await fn(); } catch (e) { fail(e); }
    finally { busy = false; b.disabled = false; b.removeAttribute('aria-busy'); paintStatus(status); }
  };
}
function confirmAction(title, copy, details, label) {
  return new Promise(resolve => {
    put('confirm-title', title); put('confirm-copy', copy); put('confirm-details', details); put('confirm-accept', label);
    const d = $('confirm-dialog');
    d.showModal();
    let done = false;
    const finish = v => { if (done) return; done = true; d.close(); resolve(v); };
    $('confirm-cancel').onclick = () => finish(false);
    $('confirm-accept').onclick = () => finish(true);
    d.oncancel = e => { e.preventDefault(); finish(false); };
  });
}

/* ── intro ────────────────────────────────────────────────────────────── */
const bootTime = performance.now();
let introClosed = false;
function finishIntro() {
  if (introClosed) return;
  introClosed = true;
  setTimeout(() => {
    $('intro')?.classList.add('leaving');
    setTimeout(() => { $('intro')?.remove(); document.body.classList.remove('booting'); }, 420);
  }, Math.max(0, 900 - (performance.now() - bootTime)));
}
$('intro-skip').onclick = finishIntro;

/* ── device status ────────────────────────────────────────────────────── */
let anchor = null;
const profileName = s => s.intro ? t('status.intro') : s.activeProfile || t('status.default');
function paintStatus(s) {
  const old = status;
  status = s;
  anchor = s.running ? { phase: s.phase, clock: s.clock || 0, at: performance.now() } : null;
  if (old.capsLock !== s.capsLock || old.activeProfile !== s.activeProfile || old.intro !== s.intro) transition();
  $('dot').classList.toggle('connected', !!s.connected);
  put('connection', s.connected ? t('conn.connected') : s.connecting ? t('conn.searching') : s.autoConnect ? t('conn.waiting') : t('conn.off'));
  put('quick-play', s.running ? t('play.pause') : t('play.play'));
  $('quick-play').disabled = !s.connected || busy;
  put('connect', s.connected ? t('conn.disconnect') : t('conn.reconnect'));
  put('active-profile', profileName(s));
  const fps = Math.round(s.actualFps || 0);
  put('native-fps', s.inputPaused ? t('status.holding') : s.running ? t('status.fps', { fps }) : t('status.stopped'));
  $('native-fps').classList.toggle('live', !!s.running && !s.inputPaused);
  put('frames', s.running ? t('status.usb', { fps, ms: (s.sendMs || 0).toFixed(1) }) : t('status.frames', { n: s.frames || 0 }));
  $('caps-state').hidden = !settings?.capsEnabled;
  put('caps-state', s.capsLock ? t('status.capsOn') : t('status.capsOff'));
  $('caps-state').classList.toggle('on', !!s.capsLock);
  put('live-label', s.inputPaused ? t('status.holding') : s.intro ? t('status.intro') : s.running ? t('status.playing') : t('status.preview'));
  $('live').classList.toggle('on', !!s.running && !s.inputPaused);
  put('input-state', s.inputPaused ? (s.capsMonitor ? t('settings.stateHolding') : t('settings.stateNoHook')) : settings?.typingPriority === false ? t('settings.stateOff') : t('settings.stateReady'));
  put('foreground-info', s.foreground ? t('profiles.foreground', { exe: s.foreground }) : t('profiles.copy'));
  for (const id of ['start', 'static', 'fw-apply', 'map-apply']) $(id).disabled = !s.connected || busy;
  $('stop').disabled = !s.running || busy;
  if ($('intro-status')) put('intro-status', s.connected ? t('intro.connected') : t('intro.waiting'));
  if (s.connected) finishIntro();
  if (s.error && s.error !== old.error) fail(s.error);
  if (map) $('map-save').disabled = !dirtyMap || busy;
  kick();
}

/* ── lighting: configuration and controls ─────────────────────────────── */
let updateTimer, tourTimer = null, effectFilter = 'all';
const selection = new Set();
const blankFx = () => ({ mode: 'solid', colorA: config.colorA, colorB: config.colorB, speed: config.speed });

function normalize(c) {
  if (!c.fps || c.fps === 120) c.fps = 30;
  c.scale ??= 50; c.angle ??= 0;
  const perKey = c.perKey || [];
  c.perKey = keys.map((_, i) => perKey[i] || null);
  return c;
}
function changed() {
  transition();
  clearTimeout(updateTimer);
  updateTimer = setTimeout(() => invoke('update', { config: clone(config) }).catch(fail), 24);
}
async function flush() { clearTimeout(updateTimer); await invoke('update', { config: clone(config) }); }

/** What the selected keys currently run: one effect, several, or the keyboard's. */
function selectionFx() {
  const list = [...selection].map(i => config.perKey[i]), first = list[0] || null;
  return { count: list.length, fx: first, mixed: list.some(o => JSON.stringify(o) !== JSON.stringify(first)) };
}
function applyToSelection(patch) {
  if (!selection.size) { toast(t('keys.pickFirst')); return false; }
  for (const i of selection) config.perKey[i] = patch ? { ...(config.perKey[i] || blankFx()), ...patch } : null;
  refreshKeyMarks(); syncControls(); changed();
  return true;
}
/** Colour and speed controls edit the keyboard effect, or the selected keys in per-key scope. */
function setValue(name, value) {
  if (scope === 'keys') { applyToSelection({ [name]: value }); return; }
  config[name] = value; syncControls(); changed();
}
function syncControls() {
  const sel = selectionFx(), source = scope === 'keys' ? sel.fx || blankFx() : config;
  for (const n of ['colorA', 'colorB', 'speed']) $(n).value = source[n];
  for (const n of ['brightness', 'fps', 'layout', 'scale', 'angle']) $(n).value = config[n];
  $('hexA').value = source.colorA.toUpperCase(); $('hexB').value = source.colorB.toUpperCase();
  put('speed-value', source.speed + '%');
  for (const n of ['brightness', 'scale']) put(n + '-value', config[n] + '%');
  put('angle-value', config.angle + '°');
  const mode = scope === 'keys' ? (sel.count && !sel.mixed && sel.fx ? sel.fx.mode : null) : config.mode;
  put('effect-name', mode ? fxName(mode) : !sel.count ? t('keys.none') : sel.mixed ? t('keys.mixed') : t('keys.follow'));
  put('mode-tag', mode ? EFFECT_BY_ID[mode][1] : t('scope.keys'));
  put('selection-summary', t(sel.count === 1 ? 'keys.count1' : 'keys.count', { n: sel.count }));
  $('paint-tools').hidden = config.mode !== 'paint';
  $('speed').disabled = scope === 'keys' ? !sel.count || (!!mode && Lights.isStatic(mode)) || (!mode && !sel.mixed) : Lights.isStatic(config.mode);
  $('key-follow').disabled = !sel.count || (!sel.fx && !sel.mixed);
  $('key-clear-all').disabled = !config.perKey.some(Boolean);
  $('reactive-hint').hidden = !(mode && Lights.isReactive(mode));
  for (const b of $$('[data-mode]')) { pressed(b, b.dataset.mode === mode); b.disabled = scope === 'keys' && b.dataset.mode === 'paint'; }
  put('library-hint', scope === 'keys' ? t('library.hintKeys') : t('library.hint'));
  if (page === 'lighting') put('keyhint', scope === 'keys' ? t('hint.keys') : config.mode === 'paint' ? t('hint.paint') : Lights.reactive(config) ? t('hint.reactive') : t('hint.lighting'));
}
function stopTour() { clearInterval(tourTimer); tourTimer = null; put('tour', t('library.tour')); }
function setEffect(mode, palette = true) {
  config.mode = mode;
  if (palette) { config.colorA = EFFECT_BY_ID[mode][2]; config.colorB = EFFECT_BY_ID[mode][3]; }
  syncControls(); changed();
}
function chooseEffect(mode) {
  stopTour();
  if (scope === 'keys') applyToSelection({ mode, colorA: EFFECT_BY_ID[mode][2], colorB: EFFECT_BY_ID[mode][3] });
  else setEffect(mode);
}
function setScope(next) {
  scope = next;
  document.body.dataset.scope = next;
  for (const b of $$('[data-scope]', $('scope-tabs'))) pressed(b, b.dataset.scope === next);
  $('select-tools').hidden = next !== 'keys' || page !== 'lighting';
  if (next === 'keys') stopTour();
  refreshKeyMarks(); syncControls();
}
for (const b of $$('[data-scope]', $('scope-tabs'))) b.onclick = () => setScope(b.dataset.scope);

/* effect library: one thumbnail canvas per card, animated only while hovered */
const THUMB_W = 152, THUMB_H = 54, THUMB_AT = .9;
let thumbRaf = 0;
function thumbConfig(mode) {
  const e = EFFECT_BY_ID[mode], [a, b] = [Lights.rgb(e[2]), Lights.rgb(e[3])];
  const colors = keys.map(k => { const box = k.layouts[config.layout], f = Math.min(1, (box.x + box.w / 2) / 760); return '#' + a.map((v, j) => Math.round(v * (1 - f) + b[j] * f).toString(16).padStart(2, '0')).join(''); });
  return { mode, colorA: e[2], colorB: e[3], brightness: 100, speed: 35, scale: 50, angle: 0, layout: config.layout, colors, perKey: [] };
}
function thumbHits(time) {
  const visible = keys.map((k, i) => i).filter(i => keys[i].layouts[config.layout].w), hits = [];
  for (let n = Math.floor(time / .45), j = 0; j < 5; j++, n--) hits.push([visible[Math.abs(Math.round(Math.sin(n * 91.7) * 9973)) % visible.length], n * .45]);
  return hits;
}
function drawThumb(canvas, time) {
  const g = canvas.getContext('2d'), c = thumbConfig(canvas.dataset.thumb);
  const colors = Lights.frame(c, keys, time, time, Lights.isReactive(c.mode) ? thumbHits(time) : []);
  g.setTransform(canvas.width / 760, 0, 0, canvas.height / 270, 0, 0);
  g.clearRect(0, 0, 760, 270);
  keys.forEach((k, i) => {
    const box = k.layouts[config.layout];
    if (!box.w) return;
    g.fillStyle = `rgb(${colors[i].map(v => Math.round(16 + led(v) * .93)).join(',')})`;
    g.beginPath(); g.roundRect(box.x + 2, box.y + 2, box.w - 4, box.h - 4, 9); g.fill();
  });
}
function drawThumbs() { for (const c of $$('canvas[data-thumb]')) drawThumb(c, THUMB_AT); }
function playThumb(canvas) {
  cancelAnimationFrame(thumbRaf);
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const start = performance.now();
  const step = now => { drawThumb(canvas, THUMB_AT + (now - start) / 1000 * .9); thumbRaf = requestAnimationFrame(step); };
  thumbRaf = requestAnimationFrame(step);
}
function stopThumb(canvas) { cancelAnimationFrame(thumbRaf); thumbRaf = 0; drawThumb(canvas, THUMB_AT); }
function buildLibrary() {
  $('effects').replaceChildren(); $('intro-mode').replaceChildren();
  const dpr = Math.min(devicePixelRatio, 2);
  for (const e of EFFECTS) {
    const card = button('', () => chooseEffect(e[0]), 'fx');
    card.dataset.mode = e[0];
    const canvas = el('canvas');
    canvas.width = THUMB_W * dpr; canvas.height = THUMB_H * dpr; canvas.dataset.thumb = e[0];
    const name = el('span', 'fx-name', fxName(e[0]));
    card.append(canvas, name);
    if (e[5]) card.append(el('span', 'fx-new', t('filter.newTag')));
    card.onpointerenter = card.onfocus = () => playThumb(canvas);
    card.onpointerleave = card.onblur = () => stopThumb(canvas);
    $('effects').append(card);
    if (!Lights.isStatic(e[0]) && !Lights.isReactive(e[0])) $('intro-mode').append(option(e[0], fxName(e[0])));
  }
  drawThumbs(); filterEffects();
}
function filterEffects() {
  const query = fold($('effect-search').value.trim());
  let shown = 0;
  for (const b of $$('[data-mode]')) {
    const e = EFFECT_BY_ID[b.dataset.mode];
    const match = (!query || fold(fxName(e[0]) + ' ' + e[0] + ' ' + e[1]).includes(query))
      && (effectFilter === 'all' || (effectFilter === 'new' ? !!e[5] : e[4] === effectFilter));
    b.hidden = !match;
    if (match) shown++;
  }
  put('effect-count', t('library.count', { n: shown, total: EFFECTS.length }));
  $('effect-empty').hidden = shown > 0;
}
$('effect-search').oninput = () => { stopTour(); filterEffects(); };
for (const b of $$('[data-filter]')) b.onclick = () => { stopTour(); effectFilter = b.dataset.filter; for (const k of $$('[data-filter]')) pressed(k, k === b); filterEffects(); };
action('tour', async () => {
  if (tourTimer) { stopTour(); return; }
  setScope('board');
  if (status.connected && !status.running) { await flush(); paintStatus(await invoke('control', { action: 'start' })); }
  const choices = EFFECTS.filter(e => !Lights.isStatic(e[0]) && !Lights.isReactive(e[0]) && !document.querySelector(`[data-mode=${e[0]}]`).hidden);
  if (!choices.length) { toast(t('library.tourEmpty')); return; }
  let i = choices.findIndex(e => e[0] === config.mode);
  const next = () => { i = (i + 1) % choices.length; setEffect(choices[i][0]); };
  next();
  tourTimer = setInterval(next, 5000);
  put('tour', t('library.tourStop'));
});

for (const suffix of ['A', 'B']) {
  $('color' + suffix).oninput = e => setValue('color' + suffix, e.target.value);
  $('hex' + suffix).onchange = e => {
    const value = e.target.value.trim();
    if (!/^#[0-9a-f]{6}$/i.test(value)) { toast(t('color.invalid'), 'error'); syncControls(); return; }
    setValue('color' + suffix, value.toLowerCase());
  };
}
$('speed').oninput = e => setValue('speed', +e.target.value);
for (const name of ['brightness', 'scale', 'angle']) $(name).oninput = e => { config[name] = +e.target.value; syncControls(); changed(); };
$('fps').onchange = () => { config.fps = +$('fps').value; changed(); };
$('layout').onchange = () => { config.layout = +$('layout').value; buildKeyboard(); drawThumbs(); changed(); };
$('swap').onclick = () => {
  if (scope === 'keys') { const f = selectionFx().fx || blankFx(); applyToSelection({ colorA: f.colorB, colorB: f.colorA }); return; }
  [config.colorA, config.colorB] = [config.colorB, config.colorA]; syncControls(); changed();
};
for (const c of ['#66d9d0', '#82b2ff', '#b7a1ff', '#f68dbb', '#ff6b6b', '#ffa46b', '#ffda85', '#9bf06b', '#ffffff', '#000000']) {
  const b = button('', () => setValue('colorA', c));
  b.style.background = c; b.setAttribute('aria-label', c); b.title = c;
  $('swatches').append(b);
}
function fill(ids, color) { keys.forEach((k, i) => { if (!ids || ids.includes(k.id)) config.colors[i] = color; }); changed(); }
$('fill').onclick = () => fill(null, config.colorA);
$('wasd').onclick = () => fill(['w', 'a', 's', 'd'], config.colorA);
$('arrows').onclick = () => fill(['up', 'down', 'left', 'right'], config.colorA);
$('clear').onclick = () => fill(null, '#000000');
$('key-follow').onclick = () => applyToSelection(null);
action('key-clear-all', async () => {
  const n = config.perKey.filter(Boolean).length;
  if (!n || !await confirmAction(t('keys.clearTitle'), t('keys.clearCopy', { n }), '', t('keys.clearAll'))) return;
  config.perKey = keys.map(() => null);
  refreshKeyMarks(); syncControls(); changed();
});

const SELECT_SETS = {
  letters: k => /^[a-z]$/.test(k.id), numbers: k => /^n\d$/.test(k.id),
  wasd: k => 'wasd'.includes(k.id) && k.id.length === 1, arrows: k => ['up', 'down', 'left', 'right'].includes(k.id),
  mods: k => ['tab', 'caps', 'lshift', 'rshift', 'lctrl', 'rctrl', 'win', 'lalt', 'ralt', 'fn'].includes(k.id),
};
for (const b of $$('[data-select]')) b.onclick = () => {
  const kind = b.dataset.select, visible = buttons.map(x => x.index), before = new Set(selection);
  selection.clear();
  for (const i of visible) {
    const on = kind === 'all' || (kind === 'invert' ? !before.has(i) : kind === 'custom' ? !!config.perKey[i] : SELECT_SETS[kind]?.(keys[i]));
    if (on) selection.add(i);
  }
  refreshKeyMarks(); syncControls();
};

function renderFirmwareEffects() {
  const at = $('fw-effect').value || 0;
  $('fw-effect').replaceChildren();
  for (let i = 0; i < 19; i++) $('fw-effect').append(option(i, t('fwfx.' + i)));
  $('fw-effect').value = at;
}
action('quick-play', async () => { stopTour(); await flush(); paintStatus(await invoke('control', { action: status.running ? 'stop' : 'start' })); });
action('connect', async () => { paintStatus(await invoke('control', { action: status.connected ? 'disconnect' : 'connect' })); toast(status.autoConnect ? t('toast.connecting') : t('toast.disconnected')); });
action('start', async () => { await flush(); paintStatus(await invoke('control', { action: 'start' })); toast(t('toast.playing'), 'success'); });
action('stop', async () => { stopTour(); paintStatus(await invoke('control', { action: 'stop' })); toast(t('toast.stopped')); });
action('static', async () => { stopTour(); await flush(); paintStatus(await invoke('control', { action: 'static' })); toast(t('toast.static'), 'success'); });
action('fw-apply', async () => { stopTour(); paintStatus(await invoke('control', { action: 'effect', effect: +$('fw-effect').value, brightness: +$('fw-brightness').value, speed: +$('fw-speed').value })); toast(t('toast.firmware'), 'success'); });

/* ── keyboard ─────────────────────────────────────────────────────────── */
let buttons = [], layer = 0, selected = -1, drag = '', erase = false;
const CODE_TO_ID = { Escape: 'esc', Minus: 'minus', Equal: 'plus', Backspace: 'bksp', Insert: 'Insert', Tab: 'tab', BracketLeft: 'lqu', BracketRight: 'rqu', Delete: 'delete', CapsLock: 'caps', Semicolon: 'sem', Quote: 'quo', Enter: 'enter', PageUp: 'pageup', PageDown: 'pagedown', ShiftLeft: 'lshift', ShiftRight: 'rshift', Comma: 'comma', Period: 'dot', Slash: 'qmark', ArrowUp: 'up', ArrowLeft: 'left', ArrowDown: 'down', ArrowRight: 'right', ControlLeft: 'lctrl', ControlRight: 'rctrl', MetaLeft: 'win', AltLeft: 'lalt', AltRight: 'ralt' };
function idsForCode(code) {
  if (/^Key[A-Z]$/.test(code)) return [code[3].toLowerCase()];
  if (/^Digit\d$/.test(code)) return ['n' + code[5]];
  if (code === 'Space') return ['mspace', 'lspace', 'rspace'];
  if (code === 'Backslash') return ['k29', 'k42'];
  return CODE_TO_ID[code] ? [CODE_TO_ID[code]] : [];
}
const keyName = i => has('key.' + keys[i].id) ? t('key.' + keys[i].id) : keys[i].label;
const layerName = n => ['Base', 'Fn 1', 'Fn 2'][n];

function buildKeyboard() {
  for (const k of $$('.key', $('keyboard'))) k.remove();
  buttons = [];
  keys.forEach((k, i) => {
    const box = k.layouts[config.layout];
    if (!box.w || !box.h) return;
    const b = button('', () => { if (page === 'remap') selectKey(i); }, 'key');
    b.dataset.keyIndex = i;
    b.style.cssText = `left:${box.x / 7.6}%;top:${box.y / 2.7}%;width:${box.w / 7.6}%;height:${box.h / 2.7}%`;
    b.append(el('span', 'key-label'), el('small', 'key-origin'));
    const paint = () => { config.colors[i] = erase ? '#000000' : config.colorA; changed(); };
    b.onpointerdown = e => {
      if (page !== 'lighting') return;
      if (scope === 'keys') {
        e.preventDefault(); b.focus();
        if (e.shiftKey || e.ctrlKey || e.metaKey) selection.has(i) ? selection.delete(i) : selection.add(i);
        else { selection.clear(); selection.add(i); drag = 'select'; }
        refreshKeyMarks(); syncControls();
      } else if (config.mode === 'paint') { e.preventDefault(); b.focus(); drag = 'paint'; erase = e.button === 2; paint(); }
      else addHit(i);
    };
    b.onpointerenter = () => {
      if (drag === 'paint') paint();
      else if (drag === 'select' && !selection.has(i)) { selection.add(i); refreshKeyMarks(); syncControls(); }
    };
    b.onkeydown = e => {
      if (page === 'remap' && e.key.startsWith('Arrow')) { e.preventDefault(); moveKey(i, e.key); return; }
      if ((e.key === 'Enter' || e.key === ' ') && page === 'lighting') {
        if (scope === 'keys') { e.preventDefault(); selection.has(i) ? selection.delete(i) : selection.add(i); refreshKeyMarks(); syncControls(); }
        else if (config.mode === 'paint') { e.preventDefault(); erase = false; paint(); }
      }
    };
    b.oncontextmenu = e => e.preventDefault();
    $('keyboard').append(b);
    buttons.push({ element: b, index: i });
  });
  if (selected >= 0 && !buttons.some(b => b.index === selected)) selected = -1;
  for (const i of [...selection]) if (!buttons.some(b => b.index === i)) selection.delete(i);
  $('key-position').replaceChildren(option('-1', t('remap.choosePosition')));
  for (const { index } of buttons) $('key-position').append(option(index, keyName(index)));
  refreshKeyboardLabels();
  if (map) { renderSelected(); renderKeycodes(); }
  fitCanvas();
}
function refreshKeyMarks() {
  for (const { element: b, index: i } of buttons) {
    const lighting = page === 'lighting';
    b.classList.toggle('picked', lighting && scope === 'keys' && selection.has(i));
    b.classList.toggle('has-fx', lighting && !!config.perKey[i]);
  }
}
function refreshKeyboardLabels() {
  const remap = page === 'remap', focused = selected >= 0 ? selected : buttons[0]?.index;
  for (const { element: b, index: i } of buttons) {
    const binding = map?.layers[layer][i], custom = remap && binding?.kind !== 'default';
    const label = remap ? bindingName(binding, keys[i].label) : keys[i].label;
    b.querySelector('.key-label').textContent = label;
    const origin = b.querySelector('.key-origin');
    origin.textContent = custom ? keys[i].label : ''; origin.hidden = !custom;
    b.classList.toggle('selected', remap && i === selected);
    b.classList.toggle('mapped', !!custom);
    b.setAttribute('aria-pressed', String(remap ? i === selected : scope === 'keys' && selection.has(i)));
    b.setAttribute('aria-label', remap ? `${keys[i].label}, ${layerName(layer)}, ${binding?.kind === 'default' ? t('remap.firmwareDefault') : label}` : keyName(i));
    b.title = remap ? `${keys[i].label} → ${bindingName(binding, t('remap.firmwareDefault'))}` : '';
    b.tabIndex = remap ? (i === focused ? 0 : -1) : 0;
  }
  refreshKeyMarks(); invalidate();
}
function moveKey(index, direction) {
  const start = keys[index].layouts[config.layout], x = start.x + start.w / 2, y = start.y + start.h / 2;
  const horizontal = direction === 'ArrowLeft' || direction === 'ArrowRight', sign = ['ArrowLeft', 'ArrowUp'].includes(direction) ? -1 : 1;
  const choices = buttons.filter(b => b.index !== index).map(b => {
    const p = keys[b.index].layouts[config.layout], dx = p.x + p.w / 2 - x, dy = p.y + p.h / 2 - y;
    const forward = (horizontal ? dx : dy) * sign, cross = Math.abs(horizontal ? dy : dx);
    return { ...b, forward, score: forward + cross * 3 };
  }).filter(b => b.forward > 1).sort((a, b) => a.score - b.score);
  if (choices.length) { selectKey(choices[0].index); choices[0].element.focus({ preventScroll: true }); choices[0].element.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
}
document.addEventListener('pointerup', () => drag = '');
window.addEventListener('blur', () => { drag = ''; if (recording) stopRecording(); });

/* ── live preview ─────────────────────────────────────────────────────── */
const canvas = $('lights'), ctx = canvas.getContext('2d', { alpha: false });
const glow = document.createElement('canvas'), glowCtx = glow.getContext('2d');
glow.width = 95; glow.height = 34;
let raf = 0, phase = 0, clock = 0, lastDraw = performance.now(), previewDirty = true;
let lastColors = [], transitionFrom = [], transitionAt = 0, ambientAt = 0, hits = [];

const hiddenNow = () => document.hidden || nativeMinimized;
const effective = () => status.running && (status.intro || status.activeProfile) && status.effective ? status.effective : config;
function kick() { if (!raf && config && !hiddenNow() && page === 'lighting') raf = requestAnimationFrame(draw); }
function invalidate() { previewDirty = true; kick(); }
function transition() { transitionFrom = lastColors.map(c => [...c]); transitionAt = performance.now(); invalidate(); }
function addHit(index) {
  if (!Lights.reactive(effective())) return;
  hits.push([index, clock]);
  if (hits.length > 16) hits.shift();
  kick();
}
document.addEventListener('keydown', e => {
  if (page !== 'lighting' || e.repeat || e.target.closest('input,select,textarea,[role=textbox]')) return;
  for (const id of idsForCode(e.code)) { const b = buttons.find(b => keys[b.index].id === id); if (b) addHit(b.index); }
});
function fitCanvas() {
  const rect = $('keyboard').getBoundingClientRect();
  if (!rect.width) return;
  const dpr = Math.min(devicePixelRatio, 2);
  canvas.width = Math.round(rect.width * dpr); canvas.height = Math.round(rect.height * dpr);
  invalidate();
}
new ResizeObserver(fitCanvas).observe($('keyboard'));

function draw(now) {
  raf = 0;
  if (!config || hiddenNow() || page !== 'lighting') return; // kick() restarts the loop
  if (now - lastDraw < 15) { raf = requestAnimationFrame(draw); return; } // 60 fps is plenty for a preview
  const dt = Math.min((now - lastDraw) / 1000, .1);
  lastDraw = now;
  const c = effective(), live = !Lights.static(c);
  if (!status.inputPaused) { phase += dt * c.speed / 50; clock += dt; }
  if (anchor) {
    const elapsed = status.inputPaused ? 0 : (now - anchor.at) / 1000, pull = 1 - Math.exp(-dt * 4);
    phase += (anchor.phase + elapsed * c.speed / 50 - phase) * pull;
  }
  const changing = now - transitionAt < 180;
  if (live || previewDirty || changing) {
    while (hits.length && clock - hits[0][1] > 30) hits.shift();
    const colors = Lights.frame(c, keys, phase, clock, hits);
    if (changing && transitionFrom.length === colors.length) {
      let a = Math.min(1, (now - transitionAt) / 180);
      a = a * a * (3 - 2 * a);
      for (let i = 0; i < colors.length; i++) for (let j = 0; j < 3; j++) colors[i][j] = Math.round(transitionFrom[i][j] * (1 - a) + colors[i][j] * a);
    }
    if (settings.capsEnabled && (status.capsLock || !settings.capsOffBase)) {
      const i = keys.findIndex(k => k.id === 'caps');
      if (i >= 0) colors[i] = Lights.rgb(status.capsLock ? settings.capsOn : settings.capsOff).map(v => Math.round(v * c.brightness / 100));
    }
    paintKeyboard(colors, c.layout);
    lastColors = colors;
    previewDirty = false;
    if (now - ambientAt > 400) { ambientAt = now; setAmbient(colors); }
  }
  if (live || changing) raf = requestAnimationFrame(draw);
}
/** Backlit look: LED light spills between dark keycaps and shines through the legends. */
function paintKeyboard(colors, layout) {
  glowCtx.fillStyle = '#000'; glowCtx.fillRect(0, 0, 95, 34);
  for (const { index } of buttons) {
    const box = keys[index].layouts[layout], c = shown[index] = colors[index].map(led);
    glowCtx.fillStyle = `rgb(${c[0]},${c[1]},${c[2]})`;
    glowCtx.fillRect(box.x / 8 - .5, box.y / 8 - .5, box.w / 8 + 1, box.h / 8 + 1);
  }
  ctx.setTransform(canvas.width / 760, 0, 0, canvas.height / 270, 0, 0);
  ctx.globalAlpha = 1;
  ctx.fillStyle = '#08090d'; ctx.fillRect(0, 0, 760, 270);
  ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = 'high';
  ctx.globalAlpha = .8; ctx.drawImage(glow, 0, 0, 760, 270);
  ctx.font = '600 11px "Segoe UI Variable Text","Segoe UI",system-ui,sans-serif';
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  for (const { index } of buttons) {
    const box = keys[index].layouts[layout], [r, g, b] = shown[index], lit = Math.max(r, g, b) / 255;
    ctx.globalAlpha = 1;
    ctx.fillStyle = `rgb(${r},${g},${b})`;
    ctx.beginPath(); ctx.roundRect(box.x, box.y, box.w, box.h, 7); ctx.fill();
    ctx.fillStyle = `rgb(${Math.round(15 + r * .16)},${Math.round(16 + g * .16)},${Math.round(22 + b * .16)})`;
    ctx.beginPath(); ctx.roundRect(box.x + 2.5, box.y + 2.5, box.w - 5, box.h - 5, 5.5); ctx.fill();
    ctx.fillStyle = `rgb(${Math.round(26 + r * .2)},${Math.round(28 + g * .2)},${Math.round(37 + b * .2)})`;
    ctx.beginPath(); ctx.roundRect(box.x + 5.5, box.y + 4, box.w - 11, box.h - 12, 4); ctx.fill();
    const k = .35 + .65 * lit;
    ctx.fillStyle = lit < .06 ? '#5d6073' : `rgb(${Math.round(r * k + 255 * (1 - k) * lit + 60 * (1 - lit))},${Math.round(g * k + 255 * (1 - k) * lit + 60 * (1 - lit))},${Math.round(b * k + 255 * (1 - k) * lit + 70 * (1 - lit))})`;
    ctx.fillText(keys[index].label, box.x + box.w / 2, box.y + box.h / 2 - 2, box.w - 12);
  }
  // Bloom: the same soft light again, added on top, so bright keys spill onto their neighbours.
  ctx.globalCompositeOperation = 'lighter'; ctx.globalAlpha = .3;
  ctx.drawImage(glow, 0, 0, 760, 270);
  ctx.globalCompositeOperation = 'source-over'; ctx.globalAlpha = 1;
}
// LEDs look brighter than their raw values on a monitor; lift the mid-tones for the preview.
const LED_CURVE = Array.from({ length: 256 }, (_, v) => Math.round(255 * Math.pow(v / 255, .62)));
const led = v => LED_CURVE[v];
const shown = [];
function setAmbient(colors) {
  let r = 0, g = 0, b = 0;
  for (const c of colors) { r += c[0]; g += c[1]; b += c[2]; }
  // Normalise so a dim frame still tints the room; a dark one fades the glow out instead.
  const peak = Math.max(r, g, b, 1), level = Math.min(1, peak / (colors.length || 1) / 60), k = 215 / peak;
  document.documentElement.style.setProperty('--ambient', `rgb(${Math.round(r * k)} ${Math.round(g * k)} ${Math.round(b * k)} / ${level.toFixed(2)})`);
}
document.addEventListener('visibilitychange', kick);

/* ── pages ────────────────────────────────────────────────────────────── */
function showPage(name) {
  if (recording) stopRecording();
  page = name;
  document.body.dataset.view = name;
  for (const b of $$('button[data-page]')) { b.classList.toggle('active', b.dataset.page === name); b.setAttribute('aria-current', b.dataset.page === name ? 'page' : 'false'); }
  for (const p of $$('.page')) p.hidden = p.id !== 'page-' + name;
  put('page-title', t('page.' + name)); put('page-description', t('page.' + name + '.hint'));
  const studio = name === 'lighting' || name === 'remap';
  $('studio').hidden = !studio;
  $('lighting-inspector').hidden = name !== 'lighting'; $('remap-inspector').hidden = name !== 'remap';
  $('remap-commit').hidden = name !== 'remap'; $('layer-tabs').hidden = name !== 'remap';
  $('scope-tabs').hidden = name !== 'lighting'; $('live').hidden = name !== 'lighting';
  $('select-tools').hidden = name !== 'lighting' || scope !== 'keys';
  if (name === 'remap') put('keyhint', t('hint.remap'));
  refreshKeyboardLabels(); syncControls(); fitCanvas();
  if (name === 'remap') { renderSelected(); renderKeycodes(); syncMapState(); }
  if (name === 'macros') renderMacros();
  document.querySelector('main').scrollTop = 0;
  kick();
}
for (const b of $$('button[data-page]')) b.onclick = () => showPage(b.dataset.page);
for (const b of $$('[data-layer]')) b.onclick = () => selectLayer(+b.dataset.layer);

/* ── presets ──────────────────────────────────────────────────────────── */
function loadConfig(next) {
  stopTour(); selection.clear();
  config = normalize(clone(next));
  syncControls(); buildKeyboard(); drawThumbs(); changed();
}
function renderPresets() {
  $('presets').replaceChildren();
  for (const p of presets) {
    const chip = button('', () => { loadConfig(p.config); $('preset-name').value = p.name; toast(t('presets.opened', { name: p.name })); }, 'preset');
    const dot = el('i');
    dot.style.background = `linear-gradient(135deg, ${p.config.colorA}, ${p.config.colorB})`;
    chip.append(dot, el('span', '', p.name));
    $('presets').append(chip);
  }
  $('rule-preset').replaceChildren(option('-1', t('profiles.current')));
  presets.forEach((p, i) => $('rule-preset').append(option(i, p.name)));
}
const currentPreset = () => ({ name: $('preset-name').value.trim(), config: clone(config) });
action('save', async () => { presets = await invoke('save_preset', { preset: currentPreset() }); renderPresets(); toast(t('presets.saved'), 'success'); });
action('delete', async () => {
  const name = $('preset-name').value.trim();
  if (!presets.some(p => p.name === name)) throw t('presets.missing');
  if (!await confirmAction(t('presets.deleteTitle'), name, t('presets.deleteNote'), t('presets.delete'))) return;
  presets = await invoke('delete_preset', { name });
  renderPresets(); toast(t('presets.deleted'));
});
action('export', async () => { if (await invoke('export_preset', { preset: currentPreset(), title: t('presets.export') })) toast(t('presets.exported'), 'success'); });
action('import', async () => {
  const p = await invoke('import_preset', { title: t('presets.import') });
  if (!p) return;
  presets = await invoke('save_preset', { preset: p });
  $('preset-name').value = p.name;
  renderPresets(); loadConfig(p.config); toast(t('presets.imported'), 'success');
});

/* ── remap: a local draft with history, written to the keyboard only on request ── */
let group = 'Basic', dirtyMap = false, savedMapSnapshot = '', appliedMapSnapshot = null, mapHistory = [], mapHistoryIndex = -1;
const mapSnapshot = () => JSON.stringify(map);
const MODS = [[1, 'Ctrl'], [2, 'Shift'], [4, 'Alt'], [8, 'Win']];
const codeLabel = c => c.code === 0 ? t('kc.0') : c.label;
function bindingName(b, fallback = '') {
  if (b.kind === 'default') return fallback || t('remap.default');
  if (b.kind === 'macro') return 'M' + (b.macroId + 1);
  const code = codes.find(c => c.code === b.code), label = code ? codeLabel(code) : String(b.code);
  return (b.kind === 'combo' ? MODS.filter(([n]) => b.mods & n).map(([, n]) => n).join('+') + '+' : '') + label;
}
function resetMapHistory() { savedMapSnapshot = mapSnapshot(); appliedMapSnapshot = null; mapHistory = [savedMapSnapshot]; mapHistoryIndex = 0; dirtyMap = false; syncMapState(); }
function markMap() {
  const next = mapSnapshot();
  if (next !== mapHistory[mapHistoryIndex]) {
    mapHistory = mapHistory.slice(0, mapHistoryIndex + 1);
    mapHistory.push(next);
    if (mapHistory.length > 60) mapHistory.shift();
    mapHistoryIndex = mapHistory.length - 1;
  }
  syncMapState(); refreshKeyboardLabels(); renderSelected(false); updatePaletteSelection();
}
function syncMapState() {
  if (!map) return;
  const current = mapSnapshot();
  dirtyMap = current !== savedMapSnapshot;
  put('map-dirty', dirtyMap ? t('remap.dirty') : t('remap.clean'));
  put('map-device-state', appliedMapSnapshot === null ? t('remap.deviceUnknown') : current === appliedMapSnapshot ? t('remap.deviceMatch') : t('remap.deviceDiffers'));
  $('map-undo').disabled = mapHistoryIndex <= 0; $('map-redo').disabled = mapHistoryIndex >= mapHistory.length - 1;
  $('map-save').disabled = !dirtyMap || busy;
  const custom = map.layers.flatMap((items, li) => items.map((binding, index) => ({ binding, index, layer: li }))).filter(x => x.binding.kind !== 'default');
  put('map-count', String(custom.length));
  $('map-changes').replaceChildren();
  if (!custom.length) $('map-changes').append(el('p', 'hint', t('remap.noneCustom')));
  for (const item of custom) {
    const b = button('', () => { showPage('remap'); selectLayer(item.layer); selectKey(item.index); buttons.find(x => x.index === item.index)?.element.scrollIntoView({ block: 'nearest', inline: 'nearest' }); });
    const from = el('span', '', layerName(item.layer) + ' / ' + keyName(item.index)), to = el('span', '', bindingName(item.binding));
    b.append(from, to);
    b.disabled = !buttons.some(x => x.index === item.index);
    b.title = from.textContent + ' → ' + to.textContent + (b.disabled ? ' · ' + t('remap.hiddenInLayout') : '');
    $('map-changes').append(b);
  }
}
function restoreMapHistory(offset) {
  const index = mapHistoryIndex + offset;
  if (index < 0 || index >= mapHistory.length || busy) return;
  stopRecording();
  mapHistoryIndex = index;
  map = JSON.parse(mapHistory[index]);
  macroIndex = Math.min(macroIndex, map.macros.length - 1);
  $('fn-mode').value = map.fnMode;
  refreshKeyboardLabels(); renderSelected(); renderKeycodes(); renderMacros(); syncMapState();
  toast(offset < 0 ? t('remap.undone') : t('remap.redone'));
}
$('map-undo').onclick = () => restoreMapHistory(-1);
$('map-redo').onclick = () => restoreMapHistory(1);
document.addEventListener('keydown', e => {
  if (page !== 'remap' || recording || e.target.closest('input,select,textarea,[role=textbox]') || document.querySelector('dialog[open]')) return;
  if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'z') { e.preventDefault(); restoreMapHistory(e.shiftKey ? 1 : -1); }
});
function selectLayer(value) {
  layer = value;
  for (const b of $$('[data-layer]')) pressed(b, +b.dataset.layer === layer);
  refreshKeyboardLabels(); renderSelected(); renderKeycodes();
}
function selectKey(index) {
  if (!buttons.some(b => b.index === index)) return;
  selected = index;
  refreshKeyboardLabels(); renderSelected(); renderKeycodes();
}
$('key-position').onchange = () => {
  const value = +$('key-position').value;
  if (value >= 0) { selectKey(value); buttons.find(b => b.index === selected)?.element.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  else { selected = -1; refreshKeyboardLabels(); renderSelected(); renderKeycodes(); }
};
for (const [id, step] of [['key-previous', -1], ['key-next', 1]]) $(id).onclick = () => {
  const at = buttons.findIndex(b => b.index === selected), next = at < 0 ? (step < 0 ? buttons.length - 1 : 0) : (at + step + buttons.length) % buttons.length;
  selectKey(buttons[next].index);
};
function renderSelected(syncModifiers = true) {
  if (!map) return;
  const binding = selected >= 0 ? map.layers[layer][selected] : null, origin = selected >= 0 ? keyName(selected) : '';
  put('selected-key', selected < 0 ? t('remap.pickKey') : origin + ' / ' + layerName(layer));
  put('binding-label', selected < 0 ? t('remap.pickKeyHint') : t('remap.assigned', { name: bindingName(binding, t('remap.firmwareDefault')) }) + (binding.kind === 'macro' ? ' · ' + (map.macros[binding.macroId]?.name || t('remap.macroGone')) : ''));
  put('selected-key-glyph', selected < 0 ? '?' : bindingName(binding, keys[selected].label));
  put('selected-key-origin', selected < 0 ? t('remap.position') : origin);
  put('binding-type', selected < 0 ? t('remap.noneSelected') : t('kind.' + binding.kind));
  $('key-position').value = selected;
  $('key-default').disabled = selected < 0 || binding?.kind === 'default';
  $('modifier-options').disabled = selected < 0;
  if (syncModifiers) for (const c of $$('[data-mod]')) c.checked = !!((binding?.kind === 'combo' ? binding.mods : 0) & +c.dataset.mod);
  updateAssignmentHint();
}
function updateAssignmentHint() {
  const mods = $$('[data-mod]:checked').map(x => x.parentElement.textContent.trim());
  put('assignment-hint', selected < 0 ? t('remap.startHint') : t('remap.assignTo', { key: keys[selected].label, layer: layerName(layer) }) + (mods.length ? ' · ' + mods.join(' + ') + ' + ' + t('remap.mainKey') : ''));
}
for (const c of $$('[data-mod]')) c.onchange = () => { updateAssignmentHint(); renderKeycodes(); };
function assign(binding) {
  if (selected < 0) { toast(t('remap.pickFirst'), 'error'); return; }
  map.layers[layer][selected] = clone(binding);
  markMap(); renderSelected();
  toast(t('remap.drafted', { layer: layerName(layer), key: keys[selected].label, name: bindingName(binding, t('remap.firmwareDefault')) }), 'success');
}
const describeCode = c => has('kc.' + c.code) ? t('kc.' + c.code) : c.group === 'Mouse' ? c.label + ' · ' + t('group.Mouse') : c.label;
function updatePaletteSelection() {
  if (!map) return;
  const binding = selected >= 0 ? JSON.stringify(map.layers[layer][selected]) : null;
  for (const b of $$('button[data-binding]', $('keycodes'))) { const on = !!binding && binding === b.dataset.binding; b.classList.toggle('is-current', on); b.setAttribute('aria-pressed', String(on)); }
}
function renderKeycodes() {
  if (!map) return;
  const q = fold($('key-search').value.trim()), mods = $$('[data-mod]:checked').reduce((a, b) => a | +b.dataset.mod, 0);
  $('keycodes').replaceChildren();
  let count = 0;
  const makeSection = (name, entries, wide = false) => {
    if (!entries.length) return;
    const section = el('section', 'keycode-section'), row = el('div', 'keycode-row' + (wide ? ' wide' : ''));
    for (const e of entries) {
      const binding = e.binding || { kind: mods ? 'combo' : 'key', code: e.code, mods, macroId: 0 };
      const valid = e.binding || !mods || (e.code >= 4 && e.code <= 115), label = e.binding ? e.label : codeLabel(e);
      const b = button(label, () => assign(binding));
      b.dataset.binding = JSON.stringify(binding);
      b.title = e.description || describeCode(e);
      b.disabled = selected < 0 || !valid;
      b.setAttribute('aria-label', label + ' — ' + b.title);
      b.onmouseenter = b.onfocus = () => put('keycode-description', valid ? b.title : t('remap.comboInvalid'));
      row.append(b); count++;
    }
    section.append(el('h3', '', name), row);
    $('keycodes').append(section);
  };
  const list = codes.filter(c => q ? fold(codeLabel(c) + ' ' + describeCode(c)).includes(q) : c.group === group);
  if (!q && group === 'Basic') {
    const rows = [['letters', 'QWERTYUIOPASDFGHJKLZXCVBNM'.split('').map(c => c.charCodeAt(0) - 65 + 4)], ['numbers', [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 53, 45, 46, 47, 48, 49, 51, 52, 54, 55, 56]], ['function', Array.from({ length: 12 }, (_, i) => i + 58)], ['navigation', null]];
    const used = new Set();
    for (const [name, order] of rows) {
      const entries = order ? order.map(n => list.find(c => c.code === n)).filter(Boolean) : list.filter(c => !used.has(c.code));
      entries.forEach(c => used.add(c.code));
      makeSection(t('remap.section.' + name), entries, name === 'navigation');
    }
  } else makeSection(q ? t('remap.results') : t('remap.section.' + group), list, group !== 'Basic');
  if (q || group === 'Macro') {
    const entries = map.macros.map((m, i) => ({ label: 'M' + (i + 1) + ' · ' + m.name, description: t('macro.summary', { n: m.steps.length, repeat: m.repeat }), binding: { kind: 'macro', code: 0, mods: 0, macroId: i } })).filter(m => !q || fold(m.label + ' macro').includes(q));
    makeSection(t('remap.section.Macro'), entries, true);
  }
  if (!count) {
    const empty = el('div', 'empty', q ? t('remap.noResults') : group === 'Macro' ? t('remap.noMacros') : t('remap.noneInGroup'));
    if (group === 'Macro' && !q) empty.append(button(t('remap.openMacros'), () => showPage('macros')));
    $('keycodes').append(empty);
  }
  put('keycode-count', t('remap.count', { n: count }));
  put('keycode-description', selected < 0 ? t('remap.descIdle') : t('remap.descReady'));
  updatePaletteSelection();
}
for (const b of $$('[data-group]')) b.onclick = () => { group = b.dataset.group; $('key-search').value = ''; for (const x of $$('[data-group]')) pressed(x, x === b); renderKeycodes(); };
$('key-search').oninput = renderKeycodes;
$('key-default').onclick = () => assign({ kind: 'default', code: 0, mods: 0, macroId: 0 });
$('fn-mode').onchange = () => { map.fnMode = +$('fn-mode').value; markMap(); };
$('layer-reset').onclick = () => { map.layers[layer] = clone(defaultMap.layers[layer]); markMap(); renderSelected(); renderKeycodes(); toast(t('remap.layerReset', { layer: layerName(layer) })); };
action('map-reset', async () => {
  if (!await confirmAction(t('remap.resetTitle'), t('remap.resetCopy'), t('remap.resetNote'), t('remap.resetAccept'))) return;
  map.layers = clone(defaultMap.layers); map.fnMode = 0; $('fn-mode').value = 0;
  markMap(); renderSelected(); renderKeycodes();
});
async function saveMap() {
  const candidate = clone(map), snapshot = JSON.stringify(candidate);
  await invoke('save_keymap', { map: candidate });
  savedMapSnapshot = snapshot; syncMapState(); toast(t('remap.saved'), 'success');
}
action('map-save', saveMap); action('macro-save', saveMap);
action('map-apply', async () => {
  stopRecording();
  const candidate = clone(map);
  const changedKeys = candidate.layers.flatMap((l, li) => l.map((b, i) => b.kind === 'default' ? null : `${layerName(li)} / ${keys[i].label} → ${bindingName(b)}`)).filter(Boolean);
  const details = [t('remap.reviewCounts', { keys: changedKeys.length, macros: candidate.macros.length }), t('remap.reviewFn', { fn: candidate.fnMode === 0 ? 'Fn 1' : 'Fn 2' }), '', changedKeys.length ? changedKeys.join('\n') : t('remap.reviewAllDefault'), '', ...candidate.macros.map((m, i) => `M${i + 1} / ${m.name} · ${t('macro.summary', { n: m.steps.length, repeat: m.repeat })}`)].join('\n');
  if (!await confirmAction(t('remap.reviewTitle'), t('remap.reviewCopy'), details, t('remap.reviewAccept'))) return;
  paintStatus(await invoke('apply_keymap', { map: candidate }));
  appliedMapSnapshot = JSON.stringify(candidate); savedMapSnapshot = appliedMapSnapshot;
  syncMapState(); toast(t('remap.written'), 'success');
});

/* ── macros ───────────────────────────────────────────────────────────── */
let macroIndex = -1, recording = false, recordTime = 0;
const held = new Set();
const activeMacro = () => map.macros[macroIndex];
const standardCodes = () => codes.filter(c => (c.code >= 4 && c.code <= 115) || (c.code >= 224 && c.code <= 231));
function renderMacros() {
  if (!map) return;
  const m = activeMacro();
  $('macro-empty').hidden = !!m; $('macro-editor').hidden = !m;
  $('macro-list').replaceChildren();
  map.macros.forEach((x, i) => $('macro-list').append(button('M' + (i + 1) + ' · ' + x.name, () => { stopRecording(); macroIndex = i; renderMacros(); }, i === macroIndex ? 'active' : '')));
  if (!m) return;
  $('macro-name').value = m.name; $('macro-repeat').value = m.repeat;
  if (!recording) put('record-area', t('macro.recordIdle'));
  renderSteps();
}
function renderSteps() {
  const m = activeMacro();
  if (!m) return;
  put('macro-count', m.steps.length + ' / 84');
  $('macro-steps').replaceChildren();
  m.steps.forEach((s, i) => {
    const tr = el('tr'), cell = child => { const td = el('td'); td.append(child); tr.append(td); };
    tr.append(el('td', '', String(i + 1)));
    const kind = el('select');
    kind.append(option('down', t('macro.press')), option('up', t('macro.release')));
    kind.value = s.down ? 'down' : 'up';
    kind.setAttribute('aria-label', t('macro.action') + ' ' + (i + 1));
    kind.onchange = () => { s.down = kind.value === 'down'; markMap(); };
    cell(kind);
    const key = el('select');
    for (const c of standardCodes()) key.append(option(c.code, c.label));
    key.value = s.code;
    key.setAttribute('aria-label', t('macro.key') + ' ' + (i + 1));
    key.onchange = () => { s.code = +key.value; markMap(); };
    cell(key);
    const delay = el('input');
    delay.type = 'number'; delay.min = 20; delay.max = 32767; delay.value = s.delay;
    delay.setAttribute('aria-label', t('macro.delay') + ' ' + (i + 1));
    delay.onchange = () => { s.delay = +delay.value; markMap(); };
    cell(delay);
    const del = button('×', () => { m.steps.splice(i, 1); markMap(); renderSteps(); }, 'icon');
    del.setAttribute('aria-label', t('macro.removeStep', { n: i + 1 }));
    cell(del);
    $('macro-steps').append(tr);
  });
}
$('macro-new').onclick = () => {
  stopRecording();
  if (map.macros.length >= 10) { toast(t('macro.max'), 'error'); return; }
  map.macros.push({ name: 'Macro ' + (map.macros.length + 1), repeat: 1, steps: [] });
  macroIndex = map.macros.length - 1;
  markMap(); renderMacros();
};
$('macro-name').onchange = () => { activeMacro().name = $('macro-name').value; markMap(); renderMacros(); };
$('macro-repeat').onchange = () => { activeMacro().repeat = +$('macro-repeat').value; markMap(); };
$('macro-add-step').onclick = () => {
  stopRecording();
  const m = activeMacro();
  if (m.steps.length > 82) { toast(t('macro.noRoom'), 'error'); return; }
  m.steps.push({ code: 4, down: true, delay: 20 }, { code: 4, down: false, delay: 50 });
  markMap(); renderSteps();
};
action('macro-delete', async () => {
  stopRecording();
  if (!await confirmAction(t('macro.deleteTitle'), activeMacro().name, t('macro.deleteNote'), t('macro.delete'))) return;
  for (const b of map.layers.flat()) if (b.kind === 'macro') { if (b.macroId === macroIndex) b.kind = 'default'; else if (b.macroId > macroIndex) b.macroId--; }
  map.macros.splice(macroIndex, 1);
  macroIndex = Math.min(macroIndex, map.macros.length - 1);
  markMap(); renderMacros();
});
const EVENT_CODES = { Enter: 40, Escape: 41, Backspace: 42, Tab: 43, Space: 44, Minus: 45, Equal: 46, BracketLeft: 47, BracketRight: 48, Backslash: 49, Semicolon: 51, Quote: 52, Backquote: 53, Comma: 54, Period: 55, Slash: 56, CapsLock: 57, PrintScreen: 70, ScrollLock: 71, Pause: 72, Insert: 73, Home: 74, PageUp: 75, Delete: 76, End: 77, PageDown: 78, ArrowRight: 79, ArrowLeft: 80, ArrowDown: 81, ArrowUp: 82, NumLock: 83, NumpadDivide: 84, NumpadMultiply: 85, NumpadSubtract: 86, NumpadAdd: 87, NumpadEnter: 88, Numpad1: 89, Numpad2: 90, Numpad3: 91, Numpad4: 92, Numpad5: 93, Numpad6: 94, Numpad7: 95, Numpad8: 96, Numpad9: 97, Numpad0: 98, NumpadDecimal: 99, ControlLeft: 224, ShiftLeft: 225, AltLeft: 226, MetaLeft: 227, ControlRight: 228, ShiftRight: 229, AltRight: 230, MetaRight: 231 };
function eventCode(e) {
  if (/^Key[A-Z]$/.test(e.code)) return e.code.charCodeAt(3) - 65 + 4;
  if (/^Digit[0-9]$/.test(e.code)) return e.code === 'Digit0' ? 39 : 29 + Number(e.code.slice(-1));
  if (/^F([1-9]|1[0-2])$/.test(e.code)) return 57 + Number(e.code.slice(1));
  return EVENT_CODES[e.code];
}
function stopRecording() {
  if (!recording) return;
  recording = false;
  const m = activeMacro();
  for (const code of held) if (m.steps.length < 84) m.steps.push({ code, down: false, delay: 20 });
  held.clear();
  $('record-area').classList.remove('recording');
  put('record-area', t('macro.recordDone'));
  $('record-stop').disabled = true; $('record-start').disabled = false;
  markMap(); renderSteps();
}
$('record-start').onclick = () => {
  stopRecording();
  if (activeMacro().steps.length > 80) { toast(t('macro.nearlyFull'), 'error'); return; }
  recording = true; held.clear(); recordTime = performance.now();
  $('record-area').classList.add('recording');
  put('record-area', t('macro.recordLive'));
  $('record-stop').disabled = false; $('record-start').disabled = true;
  $('record-area').focus();
};
$('record-stop').onclick = stopRecording;
$('record-area').onblur = stopRecording;
for (const type of ['keydown', 'keyup']) $('record-area').addEventListener(type, e => {
  if (!recording) return;
  e.preventDefault(); e.stopPropagation();
  if (e.repeat) return;
  const code = eventCode(e), down = type === 'keydown';
  if (!code || (down && held.has(code)) || (!down && !held.has(code))) return;
  const m = activeMacro();
  if (down && m.steps.length + held.size + 2 > 84) { stopRecording(); toast(t('macro.full')); return; }
  if (down) held.add(code); else held.delete(code);
  const now = performance.now();
  m.steps.push({ code, down, delay: Math.max(20, Math.min(32767, Math.round(now - recordTime))) });
  recordTime = now;
  markMap(); renderSteps();
});

/* ── settings and app profiles: every change saves itself ─────────────── */
const OPTION_CHECKS = { 'typing-priority': 'typingPriority', 'opt-intro': 'intro', 'auto-play': 'autoPlay', 'caps-enabled': 'capsEnabled', 'caps-off-base': 'capsOffBase', 'logo-motion': 'logoMotion', 'auto-profiles': 'autoProfiles' };
const OPTION_VALUES = { 'intro-mode': 'introMode', 'caps-on': 'capsOn', 'caps-off': 'capsOff' };
let optionsTimer, optionsSaving = Promise.resolve();
function syncOptions() {
  for (const id in OPTION_CHECKS) $(id).checked = settings[OPTION_CHECKS[id]] !== false && !!settings[OPTION_CHECKS[id]];
  for (const id in OPTION_VALUES) $(id).value = settings[OPTION_VALUES[id]];
  $('intro-seconds').value = settings.introSeconds;
  document.body.classList.toggle('motion-off', !settings.logoMotion);
}
function saveOptions() {
  clearTimeout(optionsTimer);
  const candidate = clone(settings);
  optionsSaving = optionsSaving.then(() => invoke('save_options', { options: candidate })).catch(fail);
  invalidate(); paintStatus(status);
  return optionsSaving;
}
const saveOptionsSoon = () => { clearTimeout(optionsTimer); optionsTimer = setTimeout(saveOptions, 350); };
for (const id in OPTION_CHECKS) $(id).onchange = () => { settings[OPTION_CHECKS[id]] = $(id).checked; syncOptions(); saveOptions(); };
for (const id in OPTION_VALUES) $(id).oninput = () => { settings[OPTION_VALUES[id]] = $(id).value; invalidate(); saveOptionsSoon(); };
$('intro-seconds').onchange = () => {
  const value = Math.max(.5, Math.min(10, +$('intro-seconds').value || 2.8));
  settings.introSeconds = value; $('intro-seconds').value = value; saveOptions();
};
$('startup').onchange = async () => {
  const enabled = $('startup').checked;
  $('startup').disabled = true;
  try { $('startup').checked = await invoke('set_startup', { enabled }); toast(enabled ? t('settings.startupOn') : t('settings.startupOff'), 'success'); }
  catch (e) { $('startup').checked = !enabled; fail(e); }
  finally { $('startup').disabled = false; }
};
function renderRules() {
  $('rules').replaceChildren();
  if (!settings.rules.length) $('rules').append(el('p', 'empty', t('profiles.empty')));
  settings.rules.forEach((r, i) => {
    const row = el('div', 'rule'), enabled = el('input');
    enabled.type = 'checkbox'; enabled.checked = r.enabled;
    enabled.setAttribute('aria-label', t('profiles.enable', { name: r.name }));
    enabled.onchange = () => { r.enabled = enabled.checked; saveOptions(); };
    const swatch = el('i');
    swatch.style.background = `linear-gradient(135deg, ${r.config.colorA}, ${r.config.colorB})`;
    const info = el('div', 'rule-info');
    info.append(el('strong', '', r.name), el('small', '', r.exe + ' · ' + (EFFECT_BY_ID[r.config.mode] ? fxName(r.config.mode) : r.config.mode)));
    const up = button('↑', () => { [settings.rules[i - 1], settings.rules[i]] = [settings.rules[i], settings.rules[i - 1]]; renderRules(); saveOptions(); }, 'icon');
    up.disabled = i === 0; up.setAttribute('aria-label', t('profiles.raise', { name: r.name })); up.title = up.getAttribute('aria-label');
    const use = button(t('profiles.useCurrent'), () => { r.config = clone(config); renderRules(); saveOptions(); toast(t('profiles.updated', { name: r.name }), 'success'); });
    const del = button('×', () => { settings.rules.splice(i, 1); renderRules(); saveOptions(); }, 'icon');
    del.setAttribute('aria-label', t('profiles.remove', { name: r.name })); del.title = del.getAttribute('aria-label');
    const controls = el('div', 'rule-controls');
    controls.append(up, use, del);
    row.append(enabled, swatch, info, controls);
    $('rules').append(row);
  });
}
action('choose-exe', async () => { const exe = await invoke('choose_exe', { title: t('profiles.browse') }); if (exe) $('rule-exe').value = exe; });
$('rule-add').onclick = () => {
  const name = $('rule-name').value.trim(), exe = $('rule-exe').value.trim();
  if (!name || !exe.toLowerCase().endsWith('.exe') || /[\\/:*?]/.test(exe)) { toast(t('profiles.invalid'), 'error'); return; }
  if (settings.rules.length >= 50) { toast(t('profiles.max'), 'error'); return; }
  const idx = +$('rule-preset').value;
  settings.rules.push({ name, exe, enabled: true, config: clone(idx < 0 ? config : presets[idx].config) });
  $('rule-name').value = ''; $('rule-exe').value = '';
  renderRules(); saveOptions(); toast(t('profiles.added', { name }), 'success');
};
const workspace = () => ({ format: 'astrolight-workspace', version: 4, config: clone(config), settings: clone(settings), keymap: clone(map), presets: clone(presets) });
action('workspace-export', async () => { stopRecording(); if (await invoke('export_workspace', { bundle: workspace(), title: t('settings.export') })) toast(t('settings.exported'), 'success'); });
action('workspace-import', async () => {
  stopRecording();
  const b = await invoke('import_workspace', { title: t('settings.import') });
  if (!b) return;
  const n = b.keymap.layers.flat().filter(x => x.kind !== 'default').length;
  if (!await confirmAction(t('settings.importTitle'), t('settings.importCopy'), t('settings.importDetails', { presets: b.presets.length, rules: b.settings.rules.length, keys: n, macros: b.keymap.macros.length }), t('settings.import'))) return;
  stopTour(); clearTimeout(updateTimer);
  await invoke('apply_workspace', { bundle: b });
  applyInitial(await invoke('init'));
  appliedMapSnapshot = null; syncMapState(); toast(t('settings.imported'), 'success');
});

/* ── boot ─────────────────────────────────────────────────────────────── */
/** Everything whose text depends on the language or on freshly loaded data. */
function renderAll() {
  buildLibrary(); renderFirmwareEffects(); buildKeyboard(); renderPresets(); renderKeycodes(); renderMacros(); renderRules();
  syncOptions(); syncMapState(); selectLayer(layer); showPage(page); paintStatus(status);
  if (!tourTimer) put('tour', t('library.tour'));
  $('intro-mode').value = settings.introMode;
}
function applyInitial(d) {
  nativeMinimized = !!d.minimized;
  keys = d.keys; settings = d.settings; map = d.keymap; defaultMap = d.default_keymap; presets = d.presets;
  config = normalize(d.config);
  status = d.status;
  macroIndex = map.macros.length ? 0 : -1;
  selection.clear();
  $('startup').checked = d.startup; $('fn-mode').value = map.fnMode;
  if (!$('preset-name').value) $('preset-name').value = t('presets.defaultName');
  setLanguage(settings.language || 'en', false);
  resetMapHistory();
  if (d.warning) fail(d.warning);
}
applyI18n();
(async () => {
  try {
    codes = await (await fetch('keycodes.json')).json();
    await window.__TAURI__.event.listen('device:status', e => paintStatus(e.payload));
    await window.__TAURI__.event.listen('window:minimized', e => { nativeMinimized = e.payload; kick(); });
    applyInitial(await invoke('init'));
    setTimeout(finishIntro, 1800);
  } catch (e) {
    fail(e);
    if ($('intro-status')) put('intro-status', t('intro.failed', { error: errText(e) }));
    setTimeout(finishIntro, 2000);
  }
})();

// Read-only handle for the offline UI tests in scripts/check-ui.cjs.
window.astrolight = { get config() { return config; }, get map() { return map; }, get settings() { return settings; }, get lang() { return lang; }, get selection() { return [...selection]; }, t };
