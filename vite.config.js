import {readFileSync} from 'node:fs';
import { defineConfig } from 'vite';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
const {words}=JSON.parse(readFileSync(new URL('./public/engine-layout.json',import.meta.url),'utf8'));
const runner=fileURLToPath(new URL('target/release/production-lab-cu-runner',import.meta.url));
export default defineConfig({plugins:[{name:'local-sbf-measurement',configureServer(server){
 const process=spawn(runner,[],{stdio:['pipe','pipe','pipe'],env:{...globalThis.process.env,RUST_LOG:'error'}});
 let pending=[],failed=false;
 const fail=()=>{failed=true;for(const res of pending.splice(0)){res.statusCode=503;res.end(JSON.stringify({error:'CU emulator unavailable. Run npm run engine, then restart Vite.'}));}};
 process.on('error',fail);process.on('exit',fail);process.stderr.on('data',()=>{});
 createInterface({input:process.stdout}).on('line',line=>{const res=pending.shift();if(res)res.end(line);});
 server.httpServer?.on('close',()=>process.kill());
 server.middlewares.use('/api/measure',(req,res)=>{
  res.setHeader('Content-Type','application/json');
  if(req.method!=='POST'){res.statusCode=405;res.end('{}');return;}
  let body='';req.on('data',chunk=>{body+=chunk;if(body.length>64000)req.destroy();});
  req.on('end',()=>{try{
   const {action,state}=JSON.parse(body);
   if(!Number.isInteger(action)||!(action>=0&&action<84||action===255||action===254)||!Array.isArray(state)||state.length!==words||state.some(x=>typeof x!=='string'||!/^\d+$/.test(x)||BigInt(x)>18446744073709551615n))throw Error('Invalid measurement input');
   if(failed){res.statusCode=503;res.end(JSON.stringify({error:'CU emulator unavailable'}));return;}
   pending.push(res);process.stdin.write([action,...state].join(' ')+'\n');
  }catch(e){res.statusCode=400;res.end(JSON.stringify({error:e.message}));}});
 });
}}]});
