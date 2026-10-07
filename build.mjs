import {spawnSync} from 'node:child_process';
import {copyFileSync,rmSync,readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {homedir} from 'node:os';
import {resolve} from 'node:path';
const root=fileURLToPath(new URL('.',import.meta.url));
const target=resolve(root,'target');
process.env.CC_aarch64_apple_darwin ??= '/usr/bin/clang';
function run(command,args,env=process.env){const result=spawnSync(command,args,{stdio:'inherit',env});if(result.error)throw result.error;if(result.status!==0)throw Error(`${command} failed (${result.status})`);}
run('rustc',['-O',resolve(root,'tools/stock_metadata.rs'),'-o',resolve(root,'tools/stock_metadata')]);
try { run(resolve(root,'tools/stock_metadata'),[resolve(root,'stock_data.rs')]); } finally { rmSync(resolve(root,'tools/stock_metadata'),{force:true}); }
run('rustc',['-O',resolve(root,'tools/constraint_groups.rs'),'-o',resolve(root,'tools/constraint_groups')]);
try { run(resolve(root,'tools/constraint_groups'),[resolve(root,'constraint_groups.rs')]); } finally { rmSync(resolve(root,'tools/constraint_groups'),{force:true}); }
run('cargo',['build','--release','--offline','--target','wasm32-unknown-unknown','--manifest-path',resolve(root,'engine/Cargo.toml')],{...process.env,CARGO_TARGET_DIR:target});
copyFileSync(resolve(target,'wasm32-unknown-unknown/release/production_lab_engine.wasm'),resolve(root,'public/engine.wasm'));
const engine=new WebAssembly.Instance(new WebAssembly.Module(readFileSync(resolve(root,'public/engine.wasm'))),{});
writeFileSync(resolve(root,'public/engine-layout.json'),JSON.stringify({words:engine.exports.state_words(),capsOffset:engine.exports.caps_offset()}));
try{
 run(resolve(homedir(),'.local/share/solana/install/active_release/bin/cargo-build-sbf'),['--offline','--manifest-path',resolve(root,'program/Cargo.toml'),'--features','benchmark','--sbf-out-dir',resolve(root,'program/target/benchmark')]);
 run('cargo',['build','--release','--offline','--manifest-path',resolve(root,'runner/Cargo.toml')],{...process.env,CARGO_TARGET_DIR:target});
}finally{rmSync(resolve(root,'program/target/benchmark/production_lab_cu-keypair.json'),{force:true});}
