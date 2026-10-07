import {readFileSync,writeFileSync} from 'node:fs';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import assert from 'node:assert/strict';
const {instance:{exports:e}}=await WebAssembly.instantiate(readFileSync(new URL('./public/engine.wasm',import.meta.url)),{});
const Q=1n<<24n,MAX=(1n<<64n)-1n;
const state=()=>new BigUint64Array(e.memory.buffer,e.state_ptr(),e.state_words());
const clock=()=>new BigUint64Array(e.memory.buffer,e.colony_clock_ptr(),e.colony_clock_words());
const homes=()=>new BigUint64Array(e.memory.buffer,e.colony_tiers_ptr(),8*5);
const child=spawn(new URL('target/release/production-lab-cu-runner',import.meta.url).pathname,[],{env:{...process.env,RUST_LOG:'error'},stdio:['pipe','pipe','inherit']});
const lines=createInterface({input:child.stdout})[Symbol.asyncIterator]();let count=0,maxCu=0;const measurements=[];
const snapshot=()=>[...state(),...clock(),...homes().slice(0,count*5)];
function reset(records){state().fill(0n);clock().fill(0n);clock()[5]=1n;homes().fill(0n);count=records.length;records.forEach((r,i)=>homes().set(r.map(BigInt),i*5));}
async function run(name,target=0n,initialize=0){
 const before=snapshot();assert.equal(e.colony_update(count,target,initialize),0,name+' WASM');const expected=snapshot().map(String);
 child.stdin.write([initialize?257:256,target,...before].join(' ')+'\n');const result=JSON.parse((await lines.next()).value);
 assert.equal(result.error,undefined,name+': '+JSON.stringify(result));assert.deepEqual(result.state,expected,name+' WASM/SBF');assert(result.totalCu<Number(process.env.CU_DIAGNOSTIC_LIMIT||200000));maxCu=Math.max(maxCu,result.totalCu);if(process.env.CU_DIAGNOSTIC_LIMIT)console.log(name,result.totalCu,result.metadataCu,result.solveCu);measurements.push({name,cu:result.totalCu,advance:result.metadataCu,refresh:result.solveCu});
}
try{
 for(const groups of [0,1,4,8]){reset(Array.from({length:groups},(_,i)=>[1,2,1,0,0]));await run('isolated init '+groups,0n,1);await run('isolated step '+groups,1n);}
 reset([[2,4,0,0,0]]);state()[2]=Q;clock()[2]=5n;await run('bootstrap',0n,1);
 assert.equal(clock()[1],6n);
 for(let t=1;t<=60;t++){await run('growth '+t,BigInt(t));}
 assert.equal(clock()[3],10n);assert.equal(homes()[1],10n);assert.equal(homes()[3],Q);
 const stable=snapshot();await run('duplicate time',clock()[0]);assert.deepEqual(snapshot(),stable);
 const solves=clock()[4];await run('stable day',86400n);assert.equal(clock()[0],86400n);assert.equal(clock()[4],solves);
 state()[2]=0n;state()[84+2]=0n;clock()[2]=0n;clock()[5]=1n;
 // Fish has raw index 2 in the catalogue.
 const data=JSON.parse(readFileSync(new URL('./public/recipes.json',import.meta.url)));const fish=data.rawResources.indexOf('Fish');state()[84+fish]=0n;
 await run('remove food',clock()[0],1);for(let t=0;t<80&&clock()[3]>4n;t++)await run('decline '+t,clock()[0]+1n);
 assert.equal(clock()[3],4n);
 reset([[20,100,5,0,0]]);state()[2]=Q;clock()[2]=5n;await run('half fish demand',0n,1);assert.equal(homes()[3],Q/2n);
 state()[84+fish]=Q;clock()[5]=1n;await run('stock supplies missing half',0n,1);assert.equal(homes()[3],Q);
 for(const groups of [1,2,4,8]){
  reset(Array.from({length:groups},(_,i)=>[10,20+i*5,i%11,0,0]));
  for(let i=0;i<84;i++)state()[i]=BigInt(1+i%5)*Q;
  clock()[2]=1000n;
  await run('full web '+groups,0n,1);
  await run('full web tick '+groups,1n);
 }
 for(let trial=0;trial<48;trial++){
  reset(Array.from({length:8},(_,i)=>[10+i*7,(10+i*7)*(2+(i+trial)%9),Math.floor((10+i*7)/2),0,0]));
  for(let i=0;i<84;i++)state()[i]=BigInt(1+(i*7+trial)%9)*Q;
  clock()[2]=BigInt(100+trial*20);
  for(let r=0;r<80;r++){
   const raw=data.rawResources.indexOf(data.resources[r]),index=raw<0?371+r:84+raw;
   if((r+trial)%3===0)state()[index]=5n*Q;
   if((r+trial)%4===0)state()[e.caps_offset()+r]=5n*Q+1n;
  }
  await run('mixed stock/caps '+trial,0n,1);for(let t=1;t<=8;t++)await run('mixed stock/caps '+trial+' tick '+t,BigInt(t));
 }
 console.log(JSON.stringify({cases:measurements.length,maxCu,wasmSbfExactMatch:true}));writeFileSync(new URL('./population-results.json',import.meta.url),JSON.stringify({maxCu,measurements},null,2));
}finally{child.stdin.end();child.kill();}
