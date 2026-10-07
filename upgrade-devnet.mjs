import fs from 'node:fs/promises';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {web3,api,base,loadAdmin} from '../interstellar-industries-client/server/chain.mjs';
const admin=await loadAdmin(),key='/Users/tedosijses/keys/interstellar_admin.json',buffer='/Users/tedosijses/keys/interstellar_deploy_buffer.json';
const bufferKey=web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(await fs.readFile(buffer,'utf8')))).publicKey;
const binary=new URL('program/target/deploy/production_lab_cu.so',import.meta.url),bytes=await fs.readFile(binary);
const report={program:api.program.toBase58(),bytes:bytes.length,startedAt:new Date().toISOString(),balanceBefore:await base.getBalance(admin.publicKey)};
const run=args=>new Promise((resolve,reject)=>{let out='';const p=spawn('solana',[...args,'--url',base.rpcEndpoint],{stdio:['ignore','pipe','pipe']});p.stdout.on('data',d=>{out+=d;process.stdout.write(d)});p.stderr.on('data',d=>process.stderr.write(d));p.on('close',code=>code?reject(Error('CLI exited '+code)):resolve(out));});
try{
 const program=await base.getAccountInfo(api.program);if(!program?.executable)throw Error('Expected deployed program');
 const pd=new web3.PublicKey(program.data.subarray(4,36)),previous=await base.getAccountInfo(pd);
 if(!new web3.PublicKey(previous.data.subarray(13,45)).equals(admin.publicKey))throw Error('Wrong authority');
 report.previousAllocation=previous.data.length-45;
 const extra=Math.max(0,bytes.length-(previous.data.length-45));
 const extend=extra;
 const needed=await base.getMinimumBalanceForRentExemption(previous.data.length+extend);
 report.additionalRent=Math.max(0,needed-previous.lamports);
 const bufferRent=await base.getMinimumBalanceForRentExemption(bytes.length+37);
 const shortfall=Math.max(0,bufferRent+report.additionalRent+5000000-await base.getBalance(admin.publicKey));
 if(shortfall){report.topupLamports=shortfall;report.topup=await run(['transfer',admin.publicKey.toBase58(),(shortfall/1e9).toFixed(9),'--keypair','/Users/tedosijses/keys/dev.json','--output','json']);}
 if(extend){report.extensionBytes=extend;report.extension=await run(['program','extend',api.program.toBase58(),String(extend),'--keypair',key,'--output','json']);}
 report.deploy=await run(['program','deploy',binary.pathname,'--program-id','/Users/tedosijses/keys/interstellar_program.json','--buffer',buffer,'--upgrade-authority',key,'--keypair',key,'--max-len',String(bytes.length),'--no-auto-extend','--use-rpc','--output','json']);
 const deployed=await base.getAccountInfo(pd);
 if(!deployed.data.subarray(45,45+bytes.length).equals(bytes)||!new web3.PublicKey(deployed.data.subarray(13,45)).equals(admin.publicKey))throw Error('Upgrade verification failed');
 report.sha256=createHash('sha256').update(bytes).digest('hex');report.verified=true;
}finally{
 const b=await base.getAccountInfo(bufferKey);
 if(b?.lamports){if(b.owner.toBase58()!=='BPFLoaderUpgradeab1e11111111111111111111111'||b.data.readUInt32LE(0)!==1)throw Error('Unexpected buffer');report.cleanup=await run(['program','close',bufferKey.toBase58(),'--authority',key,'--recipient',admin.publicKey.toBase58(),'--keypair',key,'--output','json']);}
 report.bufferRemaining=await base.getBalance(bufferKey);
 report.balanceAfter=await base.getBalance(admin.publicKey);report.cost=report.balanceBefore+(report.topupLamports||0)-report.balanceAfter;
 await fs.writeFile(new URL('upgrade-devnet.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
}
