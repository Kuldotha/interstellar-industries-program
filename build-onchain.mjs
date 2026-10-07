import {spawnSync} from 'node:child_process';
import {rmSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {homedir} from 'node:os';
const root=fileURLToPath(new URL('.',import.meta.url));
function run(command,args){const result=spawnSync(command,args,{stdio:'inherit',env:{...process.env,CC_aarch64_apple_darwin:'/usr/bin/clang'}});if(result.error)throw result.error;if(result.status!==0)throw Error(`${command} failed (${result.status})`);}
try {
 const sbf=resolve(homedir(),'.local/share/solana/install/active_release/bin/cargo-build-sbf');
 run(sbf,['--offline','--manifest-path',resolve(root,'program/Cargo.toml')]);
 run(sbf,['--offline','--manifest-path',resolve(root,'scheduler-test-double/Cargo.toml')]);
 run('cargo',['test','--offline','--manifest-path',resolve(root,'program/Cargo.toml'),'--lib']);
 run('cargo',['test','--offline','--manifest-path',resolve(root,'runner/Cargo.toml'),'game_tests','--','--nocapture']);
} finally {
 for(const name of ['program/target/deploy/production_lab_cu-keypair.json','scheduler-test-double/target/deploy/scheduler_test_double-keypair.json'])rmSync(resolve(root,name),{force:true});
}
