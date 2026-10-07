import {readFileSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const beforePath=process.argv[2];
if(!beforePath)throw Error('Pass the previous engine.wasm path');
const instantiate=async path=>(await WebAssembly.instantiate(readFileSync(path),{})).instance.exports;
const before=await instantiate(beforePath),after=await instantiate(new URL('./public/engine.wasm',import.meta.url));
const state=e=>new BigUint64Array(e.memory.buffer,e.state_ptr(),e.state_words());
assert.equal(before.state_words(),after.state_words());
const Q=1n<<24n,limit=1n<<40n;
let seed=19,cases=0;
const random=()=>seed=(Math.imul(seed,1664525)+1013904223)>>>0;
const set=(index,value)=>{state(before)[index]=value;state(after)[index]=value;};
function check(action){
 const a=before.build(action),b=after.build(action);assert.equal(b,a,`return status ${cases}`);assert.equal(b,0,`valid case ${cases}`);
 assert.deepEqual(Array.from(state(after).slice(0,531)),Array.from(state(before).slice(0,531)),`state mismatch ${cases}, action ${action}`);cases++;
}
for(let trial=0;trial<5000;trial++){
 state(before).fill(0n);state(after).fill(0n);
 for(let i=0;i<84;i++){
  const r=random();const value=trial%3===0?BigInt(r%5)*Q:trial%3===1?BigInt(r)*Q/65536n:BigInt(r%17)*Q+BigInt(random()%16777216);
  set(i,value);
 }
 set(282,trial%7===0?limit:BigInt(random()%200)*Q);
 for(let r=0;r<32;r++)if(random()%5===0)set(84+r,BigInt(1+random()%30)*Q);
 if(trial%2===1)for(let r=0;r<80;r++)if(random()%3===0)set(371+r,BigInt(1+random()%30)*Q);
 check(255);check(random()%84);
 set(282,BigInt(random()%200)*Q);check(254);
 for(let r=0;r<80;r++)if(random()%4===0)set(371+r,trial%3===0?0n:BigInt(random()%20)*Q);
 check(255);check(random()%84);
}
// Existing account caches must migrate without altering gameplay state.
state(before).fill(0n);for(let i=0;i<84;i++)state(before)[i]=Q;
state(before)[282]=limit;state(before)[381]=10n*Q;assert.equal(before.recalculate(),0);
state(after).set(state(before));check(255);
assert.equal(state(after)[531],2n);
const report={cases,exactGameplayStateMatch:true,legacyCacheMigration:true,comparedWords:531,accountWords:after.state_words()};
writeFileSync(new URL('./optimization-equality.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
