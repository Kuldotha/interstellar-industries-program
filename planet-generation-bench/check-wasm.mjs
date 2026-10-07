import{readFileSync,writeFileSync}from'node:fs';
const root=new URL('.',import.meta.url);const maps=JSON.parse(readFileSync(new URL('maps.json',root)));
const {instance:{exports:e}}=await WebAssembly.instantiate(readFileSync(new URL('wasm/target/wasm32-unknown-unknown/release/planet_generation_wasm_check.wasm',root)));
let checks=0;
for(const m of maps){new Uint8Array(e.memory.buffer,e.topology_ptr(),8+m.tiles.length*36).set(readFileSync(new URL(`fixtures/fixed-topology-${m.resolution}.bin`,root)));e.initialize(m.seed);for(let i=m.tiles.length-1;i>=0;i--){e.query(i,-1);const a=new Uint8Array(e.memory.buffer,e.state_ptr()+264,8);if(m.tiles[i].some((v,j)=>a[j]!==v))throw Error(`Mismatch ${m.seed}:${i}`);checks++;}}
writeFileSync(new URL('wasm-results.json',root),JSON.stringify({checks,matched:true}));console.log({checks,matched:true});
