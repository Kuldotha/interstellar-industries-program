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
  used+=v()[e.consumed_offset()+r];const balance=supply-used;assert.equal(BigInt.asIntN(64,v()[200+r]),balance,name+' balance '+resources[r]);
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
 const demand=(name,rate)=>{v()[e.consumption_offset()+ri(name)]=amount(rate);};
 reset();building('Fishing Dock');stock('Fish',100);limit('Fish',100);demand('Fish',.5);await run('full fish with residents');assert.equal(rate('Fishing Dock'),Q/2n);assert.equal(v()[280],MAX);
 reset();building('Fishing Dock');demand('Fish',.5);await run('fresh fish consumed directly');assert.equal(v()[e.consumed_offset()+ri('Fish')],Q/2n);
 reset();building('Fishing Dock',.25);stock('Fish',1);demand('Fish',.5);await run('fish reserve supports shortage');assert.equal(v()[280],4n*Q);
 stock('Fish',0);await run('empty fish limits fulfillment');assert.equal(v()[e.consumed_offset()+ri('Fish')],Q/4n);
 reset();building('Fishing Dock');building('Food Processor (fish)');demand('Fish',.25);await run('residents and factory share fresh fish');assert.equal(rate('Food Processor (fish)'),3n*Q/4n);
 for(let i=0;i<60;i++){reset();building('Fishing Dock',1+i%5);building('Food Processor (fish)',i%3);demand('Fish',i/10);if(i%2)stock('Fish',10);limit('Fish',10);await run('demand '+i);v()[282]=Q;await run('workers '+i,254);}
 console.log(JSON.stringify({cases,maxCu,wasmSbfExactMatch:!!child}));
}finally{if(child){child.stdin.end();child.kill();}}
