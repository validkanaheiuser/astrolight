'use strict';
const invoke=window.__TAURI__.core.invoke;
const wrapped=fn=>async(...args)=>{try{return{ok:true,value:await fn(...args)};}catch(error){return{ok:false,error:String(error)}}};
window.studio={
 init:wrapped(()=>invoke('init')),
 connect:wrapped(()=>invoke('control',{action:'connect'})),
 disconnect:wrapped(()=>invoke('control',{action:'disconnect'})),
 update:wrapped(config=>invoke('update',{config})),
 start:wrapped(()=>invoke('control',{action:'start'})),
 stop:wrapped(()=>invoke('control',{action:'stop'})),
 static:wrapped(()=>invoke('control',{action:'static'})),
 builtin:wrapped(o=>invoke('control',{action:'effect',effect:o.effect,brightness:o.brightness,speed:o.speed})),
 savePreset:wrapped(preset=>invoke('save_preset',{preset})),
 deletePreset:wrapped(name=>invoke('delete_preset',{name})),
 exportPreset:wrapped(preset=>invoke('export_preset',{preset})),
 importPreset:wrapped(async()=>{const item=await invoke('import_preset');if(!item)return null;const items=await invoke('save_preset',{preset:item});return{items,item};}),
 onStatus:fn=>window.__TAURI__.event.listen('device:status',event=>fn(event.payload))
};
