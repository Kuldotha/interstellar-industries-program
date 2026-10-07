import {appendFileSync} from 'node:fs';
import { readFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import assert from 'node:assert/strict';
const {instance:{exports:e}}=await WebAssembly.instantiate(readFileSync(new URL('./public/engine.wasm',import.meta.url)),{});
const view=()=>new BigUint64Array(e.memory.buffer,e.state_ptr(),e.state_words()),Q=1n<<24n;
const child=spawn(new URL('target/release/production-lab-cu-runner',import.meta.url).pathname,[],{env:{...process.env,RUST_LOG:'error'},stdio:['pipe','pipe','inherit']});
const lines=createInterface({input:child.stdout})[Symbol.asyncIterator]();
let cases=0,max=0;const paths={};
function reset(){view().fill(0n);view()[282]=65536n*Q;}
reset();
async function test(action){
 const before=Array.from(view(),String);
 assert.equal(action===255?e.recalculate():e.build(action),0);
 child.stdin.write([action,...before].join(' ')+'\n');
 const {value,done}=await lines.next();assert.equal(done,false);
 const result=JSON.parse(value);assert.equal(result.error,undefined,JSON.stringify({case:cases,action,...result}));
 assert.deepEqual(Array.from(view(),String),result.state);
 if(process.env.CAPTURE_PROFILE)appendFileSync(process.env.CAPTURE_PROFILE,JSON.stringify({case:cases,action,before,totalCu:result.totalCu})+'\n');
 const snapshot=Array.from(view());
 assert.equal(e.reference(),0);
 assert.deepEqual(Array.from(view().slice(116,281)),snapshot.slice(116,281),`incremental/reference case ${cases}, action ${action}`);
 view().set(snapshot);
 const path=String(view()[285]);paths[path]??={cases:0,maxCu:0,minCu:200000};paths[path].cases++;paths[path].maxCu=Math.max(paths[path].maxCu,result.totalCu);paths[path].minCu=Math.min(paths[path].minCu,result.totalCu);
 assert.ok(result.totalCu<200000);
 max=Math.max(max,result.totalCu);cases++;return result;
}
try{
 for(let i=0;i<84;i++)await test(i);
 reset();
 for(let i=83;i>=0;i--)await test(i);
 let seed=17;
 for(let i=0;i<120;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;await test(seed%84);}
 const fixtures=readFileSync(new URL('./fixtures/rates.tsv',import.meta.url),'utf8').trim().split('\n');
 for(const line of fixtures){reset();view().set(line.trim().split(/\s+/).slice(0,116).map(BigInt));await test(255);}
 reset();view()[0]=Q;await test(1);assert.equal(view()[116+1],Q);assert.equal(BigInt.asIntN(64,view()[116+84+JSON.parse(readFileSync(new URL('./public/recipes.json',import.meta.url))).resources.indexOf('Granules')]),0n);
 const raw=JSON.parse(readFileSync(new URL('./public/recipes.json',import.meta.url))).rawResources;
 reset();view()[1]=Q;view()[84+raw.indexOf('Granules')]=10n*Q;await test(255);assert.equal(view()[280],10n*Q);
 await test(0);assert.equal(view()[280],(1n<<64n)-1n);
 reset();await test(0);await test(2);await test(0);
 view()[282]=2n*Q;await test(254);await test(1);await test(2);
 view()[282]=100n*Q;await test(254);
 for(let i=0;i<120;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;if(i%5===0){view()[282]=BigInt(seed%100)*Q;await test(254);}else await test(seed%84);}
 for(let j=0;j<32;j++)view()[84+j]=10n*Q;await test(255);
 for(const workers of [0,1,17,50,100,65536]){view()[282]=BigInt(workers)*Q;await test(254);await test(30);}
 const names=JSON.parse(readFileSync(new URL('./public/recipes.json',import.meta.url))).recipes.map(r=>r.building);
 const copper=names.indexOf('Copper Mine'),silicate=names.indexOf('Silicate Mine'),electronics=names.indexOf('Electronics Fabricator');
 const iron=names.indexOf('Iron Mine'),coal=names.indexOf('Coal Mine'),steel=names.indexOf('Smelter'),plates=names.indexOf('Steel Mill');
 const solved=()=>Array.from(view().slice(287,371)).flatMap((flag,i)=>flag===1n?[names[i]]:[]);
 reset();view()[copper]=3n*Q;view()[silicate]=2n*Q;view()[electronics]=2n*Q;
 view()[iron]=Q;view()[coal]=Q;view()[steel]=Q;view()[plates]=Q;await test(255);
 const surplus=await test(copper);assert.equal(view()[285],1n);assert.deepEqual(solved(),[]);assert.equal(view()[287+copper],2n);
 reset();view()[copper]=Q;view()[silicate]=2n*Q;view()[electronics]=2n*Q;
 view()[iron]=Q;view()[coal]=Q;view()[steel]=Q;view()[plates]=Q;await test(255);
 const shortage=await test(copper);const shortageGroups=solved();assert.deepEqual(new Set(shortageGroups),new Set(['Copper Mine','Silicate Mine','Electronics Fabricator']));
 await test(names.indexOf('Spare Parts Factory'));assert.ok(solved().includes('Steel Mill'));assert.ok(solved().includes('Electronics Fabricator'));assert.ok(!solved().includes('Gold Refinery'));
 console.log(JSON.stringify({copperSurplusCu:surplus.totalCu,copperShortageCu:shortage.totalCu,shortageGroups}));
 for(let trial=0;trial<150;trial++){
  reset();
  for(let i=0;i<84;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;if((seed>>>24)%4===0)view()[i]=BigInt(1+(seed>>>8)%7)*Q;}
  seed=(Math.imul(seed,1664525)+1013904223)>>>0;view()[282]=BigInt(seed%80)*Q;
  await test(255);await test(seed%84);
 }
 const before=Array.from(view());assert.equal(e.build(84),1);assert.deepEqual(Array.from(view()),before);
 console.log(JSON.stringify({cases,maxCu:max,wasmSbfExactMatch:true,fullSolverMatch:true,paths,solSpent:0}));
}finally{child.stdin.end();child.kill();}
