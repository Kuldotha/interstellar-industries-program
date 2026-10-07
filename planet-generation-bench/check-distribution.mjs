import{readFileSync,writeFileSync}from'node:fs';
const root=new URL('.',import.meta.url);
const {instance:{exports:e}}=await WebAssembly.instantiate(readFileSync(new URL('wasm/target/wasm32-unknown-unknown/release/planet_generation_wasm_check.wasm',root)));
const topo=readFileSync(new URL('fixtures/fixed-topology-8.bin',root)),n=topo.readUInt32LE(0);
new Uint8Array(e.memory.buffer,e.topology_ptr(),topo.length).set(topo);
const rows=[];
for(let seed=0;seed<1000;seed++){
 e.initialize(seed);const tiles=[];const row={seed,land:0,water:0,green:0,polar:0,stoneDry:0,stoneGreen:0,stoneWater:0,largestGreen:0,stoneLargestGreen:0};
 for(let id=0;id<n;id++) {e.query(id,-1);const [h,s,f]=new Uint8Array(e.memory.buffer,e.state_ptr()+264,3);tiles.push([h,s,f]);row[s===0?'water':'land']++;if(s===1)row.green++;if(s===2)row.polar++;if(f===2){row[s===0?'stoneWater':'stoneDry']++;if(s===1)row.stoneGreen++;}}
 const seen=new Set();for(let id=0;id<n;id++){if(seen.has(id)||tiles[id][1]!==1)continue;let size=0,stones=0;const queue=[id];seen.add(id);while(queue.length){const j=queue.pop();size++;if(tiles[j][2]===2)stones++;for(let side=0;side<6;side++){const k=topo.readUInt32LE(20+j*36+side*4);if(k<n&&!seen.has(k)&&tiles[k][1]===1){seen.add(k);queue.push(k);}}}if(size>row.largestGreen){row.largestGreen=size;row.stoneLargestGreen=stones;}}
 rows.push(row);
}
const metrics={};for(const key of Object.keys(rows[0]).filter(k=>k!=='seed')){const a=rows.map(r=>r[key]).sort((a,b)=>a-b);metrics[key]={min:a[0],p05:a[49],median:(a[499]+a[500])/2,p95:a[949],max:a[999],mean:a.reduce((a,b)=>a+b,0)/a.length};}
const original=readFileSync(new URL('fixtures/expected-8.bin',root));let browser={water:0,land:0,stoneDry:0,stoneWater:0};for(let i=0;i<n;i++){const water=original[i*4+1]===0;browser[water?'water':'land']++;if(original[i*4+2]&2)browser[water?'stoneWater':'stoneDry']++;}
const result={resolution:8,tiles:n,seeds:1000,metrics,zeroDryStone:rows.filter(r=>!r.stoneDry).length,zeroGreenStone:rows.filter(r=>!r.stoneGreen).length,zeroStoneLargestGreen:rows.filter(r=>!r.stoneLargestGreen).length,browser,rows};
writeFileSync(new URL('distribution-results.json',root),JSON.stringify(result,null,2));console.log(JSON.stringify({...result,rows:undefined},null,2));
