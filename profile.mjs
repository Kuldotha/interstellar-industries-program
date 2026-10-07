import {spawn,spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs';
import {createInterface} from 'node:readline';
import {tmpdir,homedir} from 'node:os';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=fileURLToPath(new URL('.',import.meta.url)),temp=mkdtempSync(resolve(tmpdir(),'ii-cu-profile-'));
const sections=['stockMetadata','capacityAndWorkforceMetadata','activeGraphTraversal','solverSetup','allocation','alternativeProducers','factoryOutputs','resourceBalances','stockDrawdownAndBoundary','capacityAndCachedUpdateChecks','allocationSelection','allocationAdvance','allocationRetirement'];
function run(command,args,env=process.env){const r=spawnSync(command,args,{stdio:'inherit',env});if(r.error)throw r.error;if(r.status!==0)throw Error(`${command} exited ${r.status}`);}
process.env.CC_aarch64_apple_darwin ??= '/usr/bin/clang';
const children=[];
function runner(binary){const child=spawn(resolve(root,'target/release/production-lab-cu-runner'),[],{env:{...process.env,RUST_LOG:'error',PROFILE_BINARY:binary},stdio:['pipe','pipe','inherit']});children.push(child);const lines=createInterface({input:child.stdout})[Symbol.asyncIterator]();return async c=>{child.stdin.write([c.action,...c.before].join(' ')+'\n');const item=await lines.next();assert.equal(item.done,false);const result=JSON.parse(item.value);assert.equal(result.error,undefined,JSON.stringify(result));return result;};}
try{
 run(process.execPath,[resolve(root,'build.mjs')]);
 const suites=['test.mjs','stock-test.mjs'].map((file,i)=>{const path=resolve(temp,`suite-${i}.jsonl`);run(process.execPath,[resolve(root,file)],{...process.env,CAPTURE_PROFILE:path});return readFileSync(path,'utf8').trim().split('\n').map(JSON.parse);});
 run(resolve(homedir(),'.local/share/solana/install/active_release/bin/cargo-build-sbf'),['--offline','--manifest-path',resolve(root,'program/Cargo.toml'),'--sbf-out-dir',temp,'--features','cu-profile']);
 const maximum=a=>a.reduce((best,x)=>x.totalCu>best.totalCu?x:best);
 const cases=[{...maximum(suites[0]),name:'baseline maximum'},{...maximum(suites[1]),name:'stock maximum'},{...suites[1][0],name:'electronics stock example'},{...suites[1][1],name:'same stock mask cached'},{...maximum(suites[0].filter(c=>c.action<84)),name:'most expensive baseline build'}];
 const normal=runner(resolve(root,'program/target/benchmark/production_lab_cu')),profiled=runner(resolve(temp,'production_lab_cu'));
 const reports=[];
 for(let i=0;i<cases.length;i++){
  const c=cases[i],normalResult=await normal(c),detail=await profiled(c);assert.deepEqual(detail.state,normalResult.state);assert.equal(detail.profile.length,sections.length);
  const capacities=c.before.slice(0,84).map(x=>Number(x)/2**24);
  const report={name:c.name,testCase:c.case,action:c.action,activeGroups:capacities.filter(x=>x>0).length,nominalCapacity:capacities.reduce((a,b)=>a+b,0),availableWorkers:Number(c.before[282])/2**24,workforce:Number(detail.state[283])/2**24,allocationStages:Number(detail.state[281]),normalCu:normalResult.totalCu,instrumentedCu:detail.totalCu,instrumentationDeltaCu:detail.totalCu-normalResult.totalCu,sectionsCu:Object.fromEntries(sections.map((name,j)=>[name,detail.profile[j]])),unattributedCu:detail.totalCu-detail.profile.slice(0,10).reduce((a,b)=>a+b,0)};
  reports.push(report);console.log(JSON.stringify(report));
  if(c.name==='stock maximum')cases.push({name:'stock maximum with cached metadata',case:c.case,action:255,before:normalResult.state});
 }
 writeFileSync(resolve(root,'profile-results.json'),JSON.stringify({budgetCu:200000,stateMatches:true,sectionsIncludeTimingReadCost:true,allocationDetailNested:true,reports},null,2)+'\n');
}finally{for(const child of children){child.stdin.end();child.kill();}rmSync(temp,{recursive:true,force:true});}
