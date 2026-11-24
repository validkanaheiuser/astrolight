// Offline check: every key the interface asks for exists in English, and every other
// language covers the English keys (extra keys are allowed only for the kc.* descriptions).
const fs = require('fs'), path = require('path'), vm = require('vm'), assert = require('assert/strict');
const ui = path.resolve(__dirname, '..', 'ui'), read = f => fs.readFileSync(path.join(ui, f), 'utf8');
const context = vm.createContext({});
vm.runInContext(read('i18n.js') + read('effects.js') + ';globalThis.out={I18N,LANGUAGES,EFFECTS};', context);
const { I18N, LANGUAGES, EFFECTS } = context.out, en = I18N.en;
const used = new Set();
// Keys built at run time (t('fx.' + id)) end in a dot and are skipped; their families are added below.
for (const m of read('app.js').matchAll(/'([a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)+)'/g)) if (m[1] in en) used.add(m[1]);
for (const m of read('app.js').matchAll(/\bt\('([a-zA-Z0-9_.]*[a-zA-Z0-9_])'/g)) used.add(m[1]);
for (const m of read('index.html').matchAll(/data-i18n="([^"]+)"/g)) used.add(m[1]);
for (const m of read('index.html').matchAll(/data-i18n-attr="([^"]+)"/g)) for (const pair of m[1].split(';')) used.add(pair.split(':')[1]);
for (const e of EFFECTS) used.add('fx.' + e[0]);
for (let i = 0; i < 19; i++) used.add('fwfx.' + i);
for (const p of ['lighting', 'remap', 'macros', 'profiles', 'settings']) { used.add('page.' + p); used.add('page.' + p + '.hint'); }
for (const k of ['default', 'key', 'combo', 'macro']) used.add('kind.' + k);
for (const k of ['letters', 'numbers', 'function', 'navigation', 'Basic', 'Media', 'Mouse', 'Special', 'Macro']) used.add('remap.section.' + k);
const src = path.resolve(ui, '..', 'src-tauri', 'src');
for (const f of fs.readdirSync(src).filter(f => f.endsWith('.rs'))) for (const m of fs.readFileSync(path.join(src, f), 'utf8').matchAll(/"E:([a-z_]+)/g)) used.add('err.' + m[1]);
const problems = [];
for (const key of used) if (!(key in en)) problems.push('missing in en: ' + key);
for (const key in en) if (!used.has(key) && !/^(kc|key)\./.test(key)) problems.push('unused: ' + key);
for (const [code] of LANGUAGES) {
  assert(I18N[code], 'no dictionary for ' + code);
  for (const key in en) {
    if (!(key in I18N[code])) problems.push(`missing in ${code}: ${key}`);
    else for (const v of en[key].match(/\{\w+\}/g) || []) if (!I18N[code][key].includes(v)) problems.push(`${code}: ${key} lost ${v}`);
  }
  for (const key in I18N[code]) if (!(key in en) && !key.startsWith('kc.')) problems.push(`extra in ${code}: ${key}`);
}
if (problems.length) { console.error(problems.join('\n')); process.exit(1); }
console.log(JSON.stringify({ languages: LANGUAGES.map(l => l[0]), keys: Object.keys(en).length, used: used.size }));
