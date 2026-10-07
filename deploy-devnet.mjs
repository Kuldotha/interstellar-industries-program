import fs from 'node:fs';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
const root=new URL('.',import.meta.url),path=new URL('deployment-devnet.json',root),plan=JSON.parse(fs.readFileSync(path));
const keys='/Users/tedosijses/keys',admin=`${keys}/interstellar_admin.json`,program=`${keys}/interstellar_program.json`,buffer=`${keys}/interstellar_deploy_buffer.json`,source=`${keys}/dev.json`;
const binary=new URL('program/target/deploy/production_lab_cu.so',root).pathname;
let rpcId=0;
async function rpc(method,params){for(let attempt=0;attempt<4;attempt++){try{const r=await fetch(plan.rpc,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:++rpcId,method,params})});const v=await r.json();if(v.error)throw Error(JSON.stringify(v.error));return v.result;}catch(e){if(attempt===3)throw e;await new Promise(r=>setTimeout(r,1000*(attempt+1)));}}}
const info=async key=>(await rpc('getAccountInfo',[key,{encoding:'base64',commitment:'confirmed'}])).value;
const balance=async key=>(await rpc('getBalance',[key,{commitment:'confirmed'}])).value;
const save=()=>fs.writeFileSync(path,JSON.stringify(plan,null,2)+'\n');
const sol=lamports=>(lamports/1e9).toFixed(9);
function cli(args){return new Promise((resolve,reject)=>{const p=spawn('solana',[...args,'--url',plan.rpc],{stdio:['ignore','pipe','pipe']});let out='';p.stdout.on('data',d=>{out+=d;process.stdout.write(d)});p.stderr.on('data',d=>{process.stderr.write(d)});p.on('error',reject);p.on('close',code=>code===0?resolve(out):reject(Error(`Solana command failed: ${args[0]} ${args[1]} (${code})`)));});}
try{
 if(await info(plan.program))throw Error('Program account already exists; inspect before another deployment.');
 const shortfall=Math.max(0,plan.funding-await balance(plan.admin)-((await info(plan.buffer))?.lamports??0));
 if(shortfall){plan.transfer=await cli(['transfer',plan.admin,sol(shortfall),'--allow-unfunded-recipient','--keypair',source,'--fee-payer',source,'--output','json']);save();}
 plan.startedAt=new Date().toISOString();save();
 plan.deploymentOutput=await cli(['program','deploy',binary,'--program-id',program,'--buffer',buffer,'--upgrade-authority',admin,'--keypair',admin,'--fee-payer',admin,'--max-len',String(plan.binaryBytes),'--no-auto-extend','--use-rpc','--max-sign-attempts','3','--output','json']);
 plan.deployed=true;save();
 const p=await info(plan.program);if(!p?.executable)throw Error('Program is not executable');
 // ProgramData address is encoded in the upgradeable loader's Program state.
 const require=(await import('node:module')).createRequire(import.meta.url);const {PublicKey}=require('/Users/tedosijses/Projects/casino/private-casino-program/node_modules/@solana/web3.js');
 const dataKey=new PublicKey(Buffer.from(p.data[0],'base64').subarray(4,36)).toBase58();const d=await info(dataKey);const raw=Buffer.from(d.data[0],'base64'),local=fs.readFileSync(binary);
 if(raw.length!==45+local.length||!raw.subarray(45).equals(local))throw Error('Deployed binary does not exactly match the local binary');
 if(raw[12]!==1||new PublicKey(raw.subarray(13,45)).toBase58()!==plan.admin)throw Error('Unexpected upgrade authority');
 plan.programData=dataKey;plan.sha256=createHash('sha256').update(local).digest('hex');plan.verified=true;plan.verifiedAt=new Date().toISOString();save();console.log('Verified deployed bytes and interstellar_admin upgrade authority.');
}catch(error){plan.error=String(error);save();process.exitCode=1;console.error(plan.error);}
finally{
 try{
  const b=await info(plan.buffer);
  if(b&&b.lamports){const bytes=Buffer.from(b.data[0],'base64');if(b.owner!=='BPFLoaderUpgradeab1e11111111111111111111111'||bytes.readUInt32LE(0)!==1)throw Error('Unexpected buffer account state; refusing to close');plan.bufferCleanup=await cli(['program','close',plan.buffer,'--authority',admin,'--recipient',plan.admin,'--keypair',admin,'--output','json']);}
  plan.bufferRemaining=(await info(plan.buffer))?.lamports??0;
  plan.retainFundingOnAdmin=true;
  plan.adminBalanceAfter=await balance(plan.admin);plan.sourceBalanceAfter=await balance(plan.source);plan.netSourceDebit=plan.sourceBalanceBefore-plan.sourceBalanceAfter;save();
 }catch(error){plan.cleanupError=String(error);save();process.exitCode=1;console.error(plan.cleanupError);}
}
