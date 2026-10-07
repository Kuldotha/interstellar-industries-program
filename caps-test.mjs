import {readFileSync,writeFileSync} from 'node:fs';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import assert from 'node:assert/strict';
const {instance:{exports:e}}=await WebAssembly.instantiate(readFileSync(new URL('./public/engine.wasm',import.meta.url)),{});
const Q=1n<<24n,MAX=(1n<<64n)-1n,CAPS=e.caps_offset();
const v=()=>new BigUint64Array(e.memory.buffer,e.state_ptr(),e.state_words());
const {recipes,resources,rawResources}=JSON.parse(readFileSync(new URL('./public/recipes.json',import.meta.url)));
const bi=n=>recipes.findIndex(x=>x.building===n),ri=n=>resources.indexOf(n),si=r=>rawResources.includes(resources[r])?84+rawResources.indexOf(resources[r]):371+r;
const amount=n=>BigInt(Math.round(n*Number(Q)));
function reset(){v().fill(0n);v()[282]=65536n*Q;}
function building(n,c=1){assert(bi(n)>=0,n);v()[bi(n)]=amount(c);}
function stock(n,c){assert(ri(n)>=0,n);v()[si(ri(n))]=amount(c);}
function limit(n,c){assert(ri(n)>=0,n);v()[CAPS+ri(n)]=amount(c)+1n;}
const rate=n=>v()[116+bi(n)];
const child=process.env.NATIVE_ONLY?null:spawn(new URL('target/release/production-lab-cu-runner',import.meta.url).pathname,[],{env:{...process.env,RUST_LOG:'error'},stdio:['pipe','pipe','inherit']});
const lines=child&&createInterface({input:child.stdout})[Symbol.asyncIterator]();
let cases=0,maxCu=0;const measurements=[];
async function run(name,action=255){
 const before=Array.from(v(),String);assert.equal(e.build(action),0,`${name}: WASM`);const after=Array.from(v());
 if(child){child.stdin.write([action,...before].join(' ')+'\n');const result=JSON.parse((await lines.next()).value);assert.equal(result.error,undefined,JSON.stringify({name,...result}));assert.deepEqual(result.state,after.map(String),name);maxCu=Math.max(maxCu,result.totalCu);measurements.push({name,totalCu:result.totalCu,metadataCu:result.metadataCu,solveCu:result.solveCu});}
 assert.equal(e.reference(),0,name+' full solve');assert.deepEqual(Array.from(v().slice(116,281)),after.slice(116,281),name+' incremental equality');v().set(after);
 let next=MAX;
 for(let r=0;r<80;r++){
  let supply=0n,used=0n;for(let i=0;i<84;i++){assert(v()[116+i]<=v()[i]*v()[283]/Q);if(recipes[i].output===resources[r])supply+=v()[116+i];used+=BigInt(recipes[i].inputs.filter(x=>x===resources[r]).length)*v()[116+i];}
  const balance=supply-used;assert.equal(BigInt.asIntN(64,v()[200+r]),balance,name+' balance '+resources[r]);
  if(v()[CAPS+r]&&v()[si(r)]>=v()[CAPS+r]-1n)assert(balance<=0n,name+' overflow '+resources[r]);
  if(balance<0n){assert(v()[si(r)]>0n);const t=v()[si(r)]*Q/(-balance);if(t<next)next=t;}
  if(balance>0n&&v()[CAPS+r]){const t=(v()[CAPS+r]-1n-v()[si(r)])*Q/balance;if(t<next)next=t;}
 }
 for(let i=0;i<84;i++){
  let extra=v()[i]*v()[283]/Q-v()[116+i];const r=ri(recipes[i].output);
  if(v()[CAPS+r]&&v()[si(r)]>=v()[CAPS+r]-1n)extra=extra< -BigInt.asIntN(64,v()[200+r])?extra:-BigInt.asIntN(64,v()[200+r]);
  for(const input of new Set(recipes[i].inputs)){const r=ri(input);if(v()[si(r)]===0n){const slack=BigInt.asIntN(64,v()[200+r])/BigInt(recipes[i].inputs.filter(x=>x===input).length);extra=extra<slack?extra:slack;}}
  assert(extra<=16n,name+' unused feasible production: '+recipes[i].building+' '+extra);
 }
 assert.equal(v()[280],next,name+' boundary');cases++;
}
try{
 reset();building('Fishing Dock');building('Food Processor (fish)',.5);stock('Fish',10);limit('Fish',10);await run('fish replenishes half consumption');assert.equal(rate('Fishing Dock'),Q/2n);
 reset();v().fill(Q,0,84);for(const r of resources){stock(r,10);limit(r,10);}await run('all stores full');assert(v().slice(116,200).every(x=>x===0n));
 reset();building('Copper Mine');building('Silicate Mine');building('Electronics Fabricator');limit('Copper',0);await run('zero buffer live throughput');assert.equal(rate('Copper Mine'),Q);
 reset();building('Copper Mine');stock('Copper',10);limit('Copper',10);await run('full raw idle');assert.equal(rate('Copper Mine'),0n);
 building('Electronics Fabricator',.5);building('Silicate Mine');await run('full raw replenishment');assert.equal(rate('Copper Mine'),Q/2n);
 reset();building('Copper Mine');stock('Copper',9);limit('Copper',10);await run('fill boundary');assert.equal(v()[280],Q);
 reset();building('Copper Mine',2);building('Silicate Mine',2);building('Electronics Fabricator',2);building('Superconductor Fab',2);stock('Rare Earth Metals',10);stock('Electronics',10);limit('Electronics',10);await run('full electronics frees copper for superconductors');assert.equal(rate('Electronics Fabricator'),0n);assert.equal(rate('Superconductor Fab'),2n*Q);
 reset();building('Copper Mine',2);building('Silicate Mine',2);building('Electronics Fabricator',2);stock('Electronics',10);limit('Electronics',10);await run('full intermediate idle');assert.equal(rate('Electronics Fabricator'),0n);
 building('Assembly Plant',.5);stock('Polymers',10);await run('full intermediate replenishment');assert.equal(rate('Electronics Fabricator'),Q/2n);assert.equal(rate('Assembly Plant'),Q/2n);
 building('Electronics Fabricator',.25);await run('full intermediate drains');assert.equal(rate('Electronics Fabricator'),Q/4n);assert.equal(v()[280],20n*Q);
 await run('incremental capped producer',bi('Electronics Fabricator'));assert.equal(rate('Electronics Fabricator'),Q/2n);
 for(let r=0;r<80;r++){reset();v().fill(Q,0,84);stock(resources[r],10);limit(resources[r],10);await run('full '+resources[r]);}
 let seed=92;for(let trial=0;trial<120;trial++){
  reset();for(let i=0;i<84;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;v()[i]=BigInt((seed>>>16)%5)*Q;}
  for(let r=0;r<80;r++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;if(seed%3===0){stock(resources[r],10);limit(resources[r],seed%2===0?10:20);}}
  await run('mixed '+trial);await run('mixed build '+trial,seed%84);v()[282]=BigInt(seed%100)*Q;await run('workforce '+trial,254);
 }
 const result={cases,maxCu,wasmSbfExactMatch:!!child,measurements};if(child)writeFileSync(new URL('./caps-results.json',import.meta.url),JSON.stringify(result,null,2));console.log(JSON.stringify({cases,maxCu,wasmSbfExactMatch:!!child}));
}finally{if(child){child.stdin.end();child.kill();}}
