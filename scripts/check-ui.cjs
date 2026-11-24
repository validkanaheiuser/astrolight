// Offline UI test: fixture data and a loopback server. No HID access, no personal settings.
// Screenshots and results go to validation/ui-run/.
const { chromium } = require('playwright'), fs = require('fs'), assert = require('assert/strict'), path = require('path'), http = require('http');
const project = path.resolve(__dirname, '..'), out = path.join(project, 'validation', 'ui-run');
fs.mkdirSync(out, { recursive: true });
const initial = JSON.parse(fs.readFileSync(path.join(project, 'validation/ui-fixture.json'), 'utf8'));
const mime = { '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.html': 'text/html', '.json': 'application/json' };
const server = http.createServer((req, res) => {
  if (req.url === '/favicon.ico') { res.writeHead(204); return res.end(); }
  const file = path.resolve(project, 'ui', '.' + (req.url === '/' ? '/index.html' : req.url.split('?')[0]));
  if (!file.startsWith(path.join(project, 'ui') + path.sep)) { res.writeHead(403); return res.end(); }
  fs.readFile(file, (e, data) => {
    if (e) { res.writeHead(404); return res.end(); }
    res.setHeader('Content-Type', mime[path.extname(file)] || 'application/octet-stream'); res.end(data);
  });
});
const fakeBackend = d => {
  window.testCalls = []; window.testData = structuredClone(d);
  window.__TAURI__ = { event: { listen: async () => () => {} }, core: { invoke: async (name, args = {}) => {
    testCalls.push(name);
    const d = testData;
    if (name === 'init') return structuredClone(d);
    if (name === 'update') { d.config = args.config; return null; }
    if (name === 'save_keymap') { d.keymap = args.map; return null; }
    if (name === 'save_options') { d.settings = args.options; return null; }
    if (name === 'apply_keymap') { d.keymap = args.map; return d.status; }
    if (name === 'set_startup') { d.startup = args.enabled; return args.enabled; }
    if (name === 'control') { if (args.action === 'bad') throw 'E:not_connected'; d.status = { ...d.status, running: args.action === 'start' }; return d.status; }
    if (name === 'save_preset') { d.presets.push(args.preset); return d.presets; }
    return null;
  } } };
};
(async () => {
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const results = [];
  const step = async (name, fn) => { await fn(); results.push(name); };
  try {
    const p = await browser.newPage({ viewport: { width: 1360, height: 900 }, deviceScaleFactor: 1 });
    const errors = [];
    p.on('pageerror', e => errors.push(e.message));
    p.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    await p.addInitScript(fakeBackend, initial);
    await p.goto('http://127.0.0.1:' + server.address().port);
    await p.waitForFunction(() => window.astrolight?.config);
    await p.waitForSelector('#intro', { state: 'detached' });
    const app = fn => p.evaluate(fn);
    const shot = name => p.screenshot({ path: path.join(out, name + '.png'), fullPage: false });

    await step('starts in English with every effect listed', async () => {
      assert.equal(await app(() => astrolight.lang), 'en');
      assert.equal(await p.locator('#page-title').textContent(), 'Lighting');
      assert.equal(await p.locator('.fx').count(), 52);
      assert.equal(await p.locator('.key').count(), 67);
    });
    await step('choosing an effect updates the keyboard-wide config', async () => {
      await p.locator('[data-mode=aurora]').click();
      assert.equal(await app(() => astrolight.config.mode), 'aurora');
      assert.equal(await p.locator('#effect-name').textContent(), 'Aurora');
      await p.waitForTimeout(350);
      await shot('lighting');
    });
    await step('search and filters narrow the library', async () => {
      await p.locator('#effect-search').fill('rain');
      assert(await p.locator('.fx:visible').count() >= 3);
      await p.locator('#effect-search').fill('');
      await p.locator('[data-filter=reactive]').click();
      assert.equal(await p.locator('.fx:visible').count(), 6);
      await p.locator('[data-filter=all]').click();
    });
    await step('per-key scope assigns an effect to selected keys only', async () => {
      await p.locator('[data-scope=keys]').click();
      assert.equal(await p.locator('#effect-name').textContent(), 'No keys selected');
      await p.locator('[data-select=wasd]').click();
      assert.equal((await app(() => astrolight.selection)).length, 4);
      await p.locator('[data-mode=breathe]').click();
      const fx = await app(() => astrolight.config.perKey.filter(Boolean));
      assert.equal(fx.length, 4);
      assert.equal(fx[0].mode, 'breathe');
      assert.equal(await app(() => astrolight.config.mode), 'aurora');
      assert.equal(await p.locator('.key.has-fx').count(), 4);
      await p.locator('#speed').fill('80');
      assert.equal(await app(() => astrolight.config.perKey.find(Boolean).speed), 80);
      await p.locator('.key').first().click();
      assert.equal(await p.locator('#effect-name').textContent(), 'Follows the keyboard');
      await p.locator('.key').nth(3).click({ modifiers: ['Control'] });
      await p.locator('[data-mode=rripple]').click();
      assert.equal(await app(() => astrolight.config.perKey.filter(Boolean).length), 6);
      await p.locator('[data-select=arrows]').click();
      await p.locator('[data-mode=rainbow]').click();
      await p.waitForTimeout(350);
      await shot('per-key');
      await p.waitForFunction(() => testData.config.perKey?.filter(Boolean).length === 10);
      await p.locator('[data-select=wasd]').click();
      await p.locator('#key-follow').click();
      assert.equal(await app(() => astrolight.config.perKey.filter(Boolean).length), 6);
      await p.locator('[data-scope=board]').click();
      assert.equal(await p.locator('#effect-name').textContent(), 'Aurora');
    });
    await step('reactive effects respond to key presses in the preview', async () => {
      await p.locator('[data-mode=rripple]').click();
      await p.locator('#reactive-hint').waitFor({ state: 'visible' });
      await p.locator('body').click({ position: { x: 5, y: 5 } });
      await p.keyboard.press('g'); await p.keyboard.press('k');
      await p.waitForTimeout(260);
      await shot('reactive');
      await p.locator('[data-mode=aurora]').click();
    });
    await step('language switches at once and is saved', async () => {
      await p.locator('#language-quick').selectOption('vi');
      assert.equal(await p.locator('#page-title').textContent(), 'Ánh sáng');
      assert.equal(await p.locator('[data-mode=aurora] .fx-name').textContent(), 'Cực quang');
      await p.waitForFunction(() => testData.settings.language === 'vi');
      await shot('lighting-vi');
      await p.locator('#language-quick').selectOption('zh');
      assert.equal(await p.locator('#page-title').textContent(), '灯光');
      await shot('lighting-zh');
      await p.locator('#language-quick').selectOption('en');
      assert.equal(await p.locator('button[data-page=remap] span').textContent(), 'Key remap');
    });
    await step('backend error codes are shown as sentences', async () => {
      await app(() => window.__TAURI__.core.invoke('control', { action: 'bad' }).catch(e => document.dispatchEvent(new CustomEvent('x', { detail: e }))));
      assert.equal(await app(() => astrolight.t('err.not_connected')), 'The keyboard is not connected over USB.');
    });
    await step('remap edits a draft with undo and redo', async () => {
      await p.locator('button[data-page=remap]').click();
      assert.equal(await p.locator('#keycodes button:enabled').count(), 0);
      await p.locator('.key').first().click();
      assert.equal(await p.locator('#selected-key').textContent(), 'Esc / Base');
      await p.locator('#keycodes button').filter({ hasText: /^A$/ }).click();
      assert.equal(await app(() => astrolight.map.layers[0][0].code), 4);
      assert.match(await p.locator('#map-dirty').textContent(), /Unsaved/);
      await p.locator('#map-undo').click();
      assert.equal(await app(() => astrolight.map.layers[0][0].kind), 'default');
      await p.locator('#map-redo').click();
      assert.equal(await app(() => astrolight.map.layers[0][0].code), 4);
      await p.locator('[data-layer="1"]').click();
      await p.locator('[data-mod="1"]').check();
      await p.locator('#keycodes button').filter({ hasText: /^C$/ }).click();
      assert.equal(await app(() => astrolight.map.layers[1][0].mods), 1);
      await p.locator('.key').nth(1).click();
      assert(!await p.locator('[data-mod="1"]').isChecked());
      await p.locator('#key-search').fill('volume');
      assert(await p.locator('#keycodes button').count() >= 2);
      await p.locator('#key-search').fill('nosuchfunction');
      assert.match(await p.locator('#keycodes').textContent(), /Nothing matches/);
      await p.locator('#key-search').fill('');
      await p.locator('[data-layer="0"]').click();
      await shot('remap');
    });
    await step('macros can be created and assigned', async () => {
      await p.locator('[data-group=Macro]').click();
      await p.getByText('Open the macro editor', { exact: true }).click();
      await p.locator('#macro-new').click();
      await p.locator('#macro-add-step').click();
      assert.equal(await p.locator('#macro-steps tr').count(), 2);
      await shot('macros');
      await p.locator('button[data-page=remap]').click();
      await p.locator('[data-group=Macro]').click();
      await p.locator('.key').nth(2).click();
      await p.locator('#keycodes button').filter({ hasText: /^M1/ }).click();
      assert.equal(await app(() => astrolight.map.layers[0][2].kind), 'macro');
    });
    await step('saving and writing the draft goes through review', async () => {
      await p.locator('#map-save').click();
      await p.waitForFunction(() => testCalls.includes('save_keymap'));
      assert.match(await p.locator('#map-dirty').textContent(), /saved/);
      await p.locator('#map-apply').click();
      assert.match(await p.locator('#confirm-details').textContent(), /Base \/ Esc → A/);
      await p.locator('#confirm-cancel').click();
      assert(!await app(() => testCalls.includes('apply_keymap')));
      await p.locator('#map-apply').click();
      await p.locator('#confirm-accept').click();
      await p.waitForFunction(() => testCalls.includes('apply_keymap'));
    });
    await step('profiles and settings save on change', async () => {
      await p.locator('button[data-page=profiles]').click();
      await p.locator('#rule-name').fill('Game night');
      await p.locator('#rule-exe').fill('game.exe');
      await p.locator('#rule-add').click();
      await p.waitForFunction(() => testData.settings.rules.length === 1);
      await shot('profiles');
      await p.locator('button[data-page=settings]').click();
      await p.locator('#caps-enabled').check();
      await p.waitForFunction(() => testData.settings.capsEnabled === true);
      await shot('settings');
    });
    await step('narrow window keeps everything reachable', async () => {
      await p.setViewportSize({ width: 820, height: 760 });
      await p.locator('button[data-page=lighting]').click();
      await p.waitForTimeout(300);
      assert(await p.locator('#quick-play').isVisible());
      assert.equal(await app(() => document.documentElement.scrollWidth <= innerWidth), true);
      await shot('narrow');
    });
    assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(out, 'results.json'), JSON.stringify({ passed: results, hardwareWrites: 0 }, null, 1));
    console.log(JSON.stringify({ passed: results.length, hardwareWrites: 0 }));
  } finally { await browser.close(); server.close(); }
})().catch(e => { console.error(e); process.exit(1); });
