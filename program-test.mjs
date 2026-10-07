import{spawnSync}from'node:child_process';
import{fileURLToPath}from'node:url';
import{resolve}from'node:path';
const root=fileURLToPath(new URL('.',import.meta.url));
const result=spawnSync('cargo',['test','--release','--offline','--manifest-path',resolve(root,'runner/Cargo.toml'),'solarium_dispatch_and_account_validation'],{stdio:'inherit',env:{...process.env,CARGO_TARGET_DIR:resolve(root,'target'),CC_aarch64_apple_darwin:'/usr/bin/clang',RUST_LOG:'error'}});
if(result.error)throw result.error;
if(result.status!==0)throw Error(`Program tests failed (${result.status})`);
