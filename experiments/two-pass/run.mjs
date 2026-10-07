import{createInterface}from'node:readline';
import{readFileSync,writeFileSync,mkdtempSync,copyFileSync,rmSync,mkdirSync}from'node:fs';import{spawnSync,spawn}from'node:child_process';import{tmpdir,homedir}from'node:os';import{resolve}from'node:path';import{fileURLToPath}from'node:url';
const here=fileURLToPath(new URL('.',import.meta.url)),root=resolve(here,'../..'),temp=mkdtempSync(resolve(tmpdir(),'ii-two-pass-'));
function run(c,a){const x=spawnSync(c,a,{stdio:'inherit'});if(x.status!==0)throw Error(`${c}: ${x.status}`);}
try{
 for(const f of ['shared.rs','active_kernel.rs','incremental_kernel.rs','stock_metadata.rs','stock_data.rs','data.rs','components.rs','profile.rs'])copyFileSync(resolve(root,f),resolve(temp,f));
 let s=readFileSync(resolve(temp,'active_kernel.rs'),'utf8'),start=s.indexOf('#[inline(never)]\nfn allocate_values'),end=s.indexOf('#[inline(never)]\nfn mul_div',start);s=s.slice(0,start)+readFileSync(resolve(here,'allocation.rs'),'utf8')+'\n'+s.slice(end);writeFileSync(resolve(temp,'active_kernel.rs'),s);
 s=readFileSync(resolve(temp,'shared.rs'),'utf8').replace('solve_selected(mask,&cap,stock,output)?;','active::solve_active(&[true;N],&[true;R],&cap,stock,output)?;');writeFileSync(resolve(temp,'shared.rs'),s);
 s=readFileSync(resolve(here,'main.rs'),'utf8').replaceAll('BASELINE_SHARED',resolve(root,'shared.rs')).replaceAll('FIXTURES_PATH',resolve(root,'fixtures/rates.tsv'));writeFileSync(resolve(temp,'main.rs'),s);
 run('rustc',['-O','--edition=2021',resolve(temp,'main.rs'),'-o',resolve(temp,'test')]);run(resolve(temp,'test'),[resolve(here,'results.txt')]);
 if(process.argv.includes('--cu')){
  mkdirSync(resolve(temp,'program/src'),{recursive:true});for(const f of ['Cargo.toml','Cargo.lock','src/lib.rs'])copyFileSync(resolve(root,'program',f),resolve(temp,'program',f));
  run(resolve(homedir(),'.local/share/solana/install/active_release/bin/cargo-build-sbf'),['--offline','--manifest-path',resolve(temp,'program/Cargo.toml'),'--sbf-out-dir',resolve(temp,'deploy')]);
  const {words}=JSON.parse(readFileSync(resolve(root,'public/engine-layout.json'))),Q=1n<<24n;
  const blank=()=>{let a=Array(words).fill('0');a[282]=String(65536n*Q);return a;};const cases=[];
  let a=blank();for(const [i,n]of [[30,10],[31,1],[32,5],[51,10],[77,10]])a[i]=String(BigInt(n)*Q);cases.push({name:'known local bottleneck',before:a});
  a=blank();a.fill(String(100n*Q),0,84);cases.push({name:'full web 100 per recipe',before:a});
  a=[...a];a[371+10]=String(10n*Q);cases.push({name:'full web with carbon strands stock',before:a});
  const fixtures=readFileSync(resolve(root,'fixtures/rates.tsv'),'utf8').trim().split('\n');for(const index of [3,105]){a=blank();a.splice(0,116,...fixtures[index].trim().split(/\s+/).slice(0,116));cases.push({name:'fixture '+index,before:a});}
  const reports=[];for(const c of cases){const report={name:c.name};for(const kind of ['baseline','twoPass']){const child=spawn(resolve(root,'target/release/production-lab-cu-runner'),[],{env:{...process.env,RUST_LOG:'error',PROFILE_BINARY:kind==='baseline'?resolve(root,'program/target/deploy/production_lab_cu'):resolve(temp,'deploy/production_lab_cu')},stdio:['pipe','pipe','inherit']});try{const lines=createInterface({input:child.stdout})[Symbol.asyncIterator]();child.stdin.end([255,...c.before].join(' ')+'\n');const result=JSON.parse((await lines.next()).value);report[kind]={cu:result.totalCu,error:result.error,stages:result.state?.[281]};}finally{child.kill();}}reports.push(report);console.log(JSON.stringify(report));}writeFileSync(resolve(here,'results-cu.json'),JSON.stringify(reports,null,2));
 }

}finally{rmSync(temp,{recursive:true,force:true});}
