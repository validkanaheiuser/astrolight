// Offline parity check: no keyboard connection or HID writes.
const {spawnSync}=require('child_process'),fs=require('fs'),path=require('path'),vm=require('vm'),assert=require('assert/strict');
const root=path.resolve(__dirname,'..');
const run=spawnSync('cargo',['run','--locked','--quiet','--bin','effects-fixture'],{cwd:path.join(root,'src-tauri'),encoding:'utf8',maxBuffer:16*1024*1024});
if(run.status!==0)throw Error(run.stderr);
const fixture=JSON.parse(run.stdout),context=vm.createContext({});vm.runInContext(fs.readFileSync(path.join(root,'ui/effects.js'),'utf8')+';globalThis.testLights=Lights;',context);
let channels=0,maxDelta=0;
for(const item of fixture.cases){const actual=context.testLights.frame(item.config,fixture.keys,item.phase);for(let i=0;i<actual.length;i++)for(let j=0;j<3;j++){assert(Number.isFinite(actual[i][j]));const delta=Math.abs(actual[i][j]-item.frame[i][j]);maxDelta=Math.max(delta,maxDelta);assert(delta<=1,item.config.mode+' differs at '+i+','+j+': '+delta);channels++;}}
console.log(JSON.stringify({cases:fixture.cases.length,channels,maxDelta,hardwareWrites:0}));
