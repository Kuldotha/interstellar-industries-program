#![allow(dead_code)]
include!("../../shared.rs");
use std::io::{self,BufRead,Write};
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta,Instruction};
use solana_pubkey::Pubkey;
fn main(){
 let program=Pubkey::new_from_array([47;32]);let address=Pubkey::new_from_array([48;32]);
 let binary=std::env::var("PROFILE_BINARY").unwrap_or_else(|_|concat!(env!("CARGO_MANIFEST_DIR"),"/../program/target/benchmark/production_lab_cu").to_owned());
 let mut vm=Mollusk::new(&program,&binary);
 vm.compute_budget.compute_unit_limit=std::env::var("CU_DIAGNOSTIC_LIMIT").ok().and_then(|s|s.parse().ok()).unwrap_or(200_000);
 for line in io::stdin().lock().lines(){
  let Ok(line)=line else{break};
  let Ok(v)=line.split_whitespace().map(str::parse::<u64>).collect::<Result<Vec<_>,_>>() else {println!("{{\"error\":\"Invalid input\"}}");continue};
  let colony=v.first().is_some_and(|&a|a==256||a==257);let offset=if colony{2}else{1};
  if (!colony&&(v.len()!=WORDS+1 || (v[0]>=N as u64 && v[0]!=255 && v[0]!=254)))||(colony&&v.len()<2+WORDS+crate::colony::CLOCK_WORDS){println!("{{\"error\":\"Invalid input\"}}");continue;}
  let mut account=Account{lamports:1_000_000_000,data:vec![0;(v.len()-offset)*8],owner:program,executable:false,rent_epoch:0};
  for (i,&value) in v[offset..].iter().enumerate(){account.data[i*8..i*8+8].copy_from_slice(&value.to_le_bytes());}
  if colony{vm.sysvars.clock.unix_timestamp=v[1] as i64;}
  let ix=Instruction{program_id:program,accounts:vec![AccountMeta::new(address,false)],data:{if colony{let mut bytes=1u64.to_le_bytes().to_vec();bytes.push(u8::from(v[0]==257));bytes}else{let mut bytes=0u64.to_le_bytes().to_vec();bytes.push(v[0] as u8);bytes}}};
  let result=vm.process_instruction(&ix,&[(address,account)]);
  if result.raw_result.is_err(){println!("{{\"error\":\"SBF execution failed: {:?}\",\"totalCu\":{}}}",result.raw_result,result.compute_units_consumed);continue;}
  let meta=u64::from_le_bytes(result.return_data[..8].try_into().unwrap());
  let solve=u64::from_le_bytes(result.return_data[8..16].try_into().unwrap());
  let data=&result.resulting_accounts.iter().find(|(k,_)|*k==address).unwrap().1.data;
  let words=data.chunks_exact(8).map(|b|format!("\"{}\"",u64::from_le_bytes(b.try_into().unwrap()))).collect::<Vec<_>>().join(",");
  let profile=result.return_data[16..].chunks_exact(8).map(|b|u64::from_le_bytes(b.try_into().unwrap()).to_string()).collect::<Vec<_>>().join(",");
  println!("{{\"profile\":[{profile}],\"metadataCu\":{meta},\"solveCu\":{solve},\"totalCu\":{},\"state\":[{words}]}}",result.compute_units_consumed);
  io::stdout().flush().unwrap();
 }
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn solarium_dispatch_and_account_validation(){
  let program=Pubkey::new_from_array([47;32]);let address=Pubkey::new_from_array([48;32]);
  let binary=concat!(env!("CARGO_MANIFEST_DIR"),"/../program/target/benchmark/production_lab_cu");
  let mut vm=Mollusk::new(&program,binary);vm.compute_budget.compute_unit_limit=200_000;
  let payload=|tag:u64,action:u8|{let mut data=tag.to_le_bytes().to_vec();data.push(action);data};
  let account=Account{lamports:1_000_000_000,data:vec![0;WORDS*8],owner:program,executable:false,rent_epoch:0};
  let valid=Instruction{program_id:program,accounts:vec![AccountMeta::new(address,false)],data:payload(0,0)};
  let result=vm.process_instruction(&valid,&[(address,account.clone())]);
  assert!(result.raw_result.is_ok(),"{:?}",result.raw_result);
  assert_eq!(u64::from_le_bytes(result.resulting_accounts.iter().find(|(k,_)|*k==address).unwrap().1.data[..8].try_into().unwrap()),Q);
  for (name,data,writable,owner,length,missing) in [
   ("empty instruction",vec![],true,program,WORDS*8,false),
   ("missing action",0u64.to_le_bytes().to_vec(),true,program,WORDS*8,false),
   ("unknown instruction",payload(1,0),true,program,WORDS*8,false),
   ("invalid action",payload(0,84),true,program,WORDS*8,false),
   ("read-only",payload(0,0),false,program,WORDS*8,false),
   ("wrong owner",payload(0,0),true,Pubkey::new_from_array([49;32]),WORDS*8,false),
   ("wrong size",payload(0,0),true,program,WORDS*8-8,false),
   ("missing account",payload(0,0),true,program,WORDS*8,true),
  ] {
   let mut state=account.clone();state.owner=owner;state.data.resize(length,0);
   let accounts=if missing{vec![]}else{vec![AccountMeta{pubkey:address,is_signer:false,is_writable:writable}]};
   let ix=Instruction{program_id:program,accounts,data};
   let result=vm.process_instruction(&ix,&[(address,state)]);
   assert!(result.raw_result.is_err(),"{name} was accepted");
  }
 }
}
#[cfg(test)]
#[path="../../program/src/game/state.rs"]
mod world;
#[cfg(test)]
mod game_tests;
