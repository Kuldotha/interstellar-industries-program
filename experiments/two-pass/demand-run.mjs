import{readFileSync,writeFileSync,mkdtempSync,copyFileSync,rmSync}from'node:fs';import{spawnSync}from'node:child_process';import{tmpdir}from'node:os';import{resolve}from'node:path';import{fileURLToPath}from'node:url';
const here=fileURLToPath(new URL('.',import.meta.url)),root=resolve(here,'../..'),temp=mkdtempSync(resolve(tmpdir(),'ii-demand-sweeps-'));
try{
 for(const f of ['shared.rs','active_kernel.rs','incremental_kernel.rs','stock_metadata.rs','stock_data.rs','data.rs','components.rs','profile.rs'])copyFileSync(resolve(root,f),resolve(temp,f));
 writeFileSync(resolve(temp,'active_kernel.rs'),readFileSync(resolve(temp,'active_kernel.rs'),'utf8')+'\n'+readFileSync(resolve(here,'demand-sweep.rs'),'utf8'));
 copyFileSync(resolve(here,'demand-main.rs'),resolve(temp,'main.rs'));
 const build=spawnSync('rustc',['-O','--edition=2021',resolve(temp,'main.rs'),'-o',resolve(temp,'test')],{stdio:'inherit'});if(build.status)throw Error('compile failed');
 const run=spawnSync(resolve(temp,'test'),[],{encoding:'utf8'});writeFileSync(resolve(here,'demand-results.txt'),run.stdout+run.stderr);console.log(run.stdout);if(run.status)throw Error(run.stderr);
}finally{rmSync(temp,{recursive:true,force:true});}
