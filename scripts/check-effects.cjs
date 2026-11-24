// Offline parity check: no keyboard connection or HID writes.
// Runs the Rust fixture generator and replays every case through ui/effects.js.
const { spawnSync } = require('child_process'), fs = require('fs'), path = require('path'), vm = require('vm'), assert = require('assert/strict');
const root = path.resolve(__dirname, '..');
const run = spawnSync('cargo', ['run', '--locked', '--quiet', '--bin', 'effects-fixture'], { cwd: path.join(root, 'src-tauri'), encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
if (run.status !== 0) throw Error(run.stderr);
const fixture = JSON.parse(run.stdout), context = vm.createContext({});
vm.runInContext(fs.readFileSync(path.join(root, 'ui/effects.js'), 'utf8') + ';globalThis.testLights=Lights;globalThis.testEffects=EFFECTS;', context);
const modes = new Set(fixture.cases.map(c => c.config.mode));
for (const e of context.testEffects) assert(modes.has(e[0]), 'effect missing from the Rust engine: ' + e[0]);
assert.equal(modes.size, context.testEffects.length, 'the Rust engine has modes the UI does not list');
let channels = 0, maxDelta = 0, perKey = 0;
for (const item of fixture.cases) {
  if (item.config.perKey.length) perKey++;
  const actual = context.testLights.frame(item.config, fixture.keys, item.phase, item.clock, item.hits);
  for (let i = 0; i < actual.length; i++) for (let j = 0; j < 3; j++) {
    assert(Number.isFinite(actual[i][j]));
    const delta = Math.abs(actual[i][j] - item.frame[i][j]);
    maxDelta = Math.max(delta, maxDelta);
    assert(delta <= 1, `${item.config.mode} (per-key ${item.config.perKey[i]?.mode || 'none'}) differs at ${i},${j}: ${delta}`);
    channels++;
  }
}
console.log(JSON.stringify({ modes: modes.size, cases: fixture.cases.length, perKeyCases: perKey, channels, maxDelta, hardwareWrites: 0 }));
