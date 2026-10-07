#![no_std]
use pinocchio::{AccountView,Address,ProgramResult,error::ProgramError};
pinocchio::program_entrypoint!(process,2);
pinocchio::nostd_panic_handler!();pinocchio::no_allocator!();
fn process(program:&Address,accounts:&mut[AccountView],data:&[u8])->ProgramResult{
 let [authority,context]=accounts else{return Err(ProgramError::NotEnoughAccountKeys);};
 if !authority.is_signer()||!context.is_writable()||!context.owned_by(program)||data.len()<4||data.len()>512{return Err(ProgramError::InvalidInstructionData);}
 if data[..4]!=6u32.to_le_bytes()&&data[..4]!=7u32.to_le_bytes(){return Err(ProgramError::InvalidInstructionData);}
 let mut bytes=context.try_borrow_mut()?;let count=u64::from_le_bytes(bytes[..8].try_into().unwrap());bytes[..8].copy_from_slice(&(count+1).to_le_bytes());bytes[8..16].copy_from_slice(&(data.len()as u64).to_le_bytes());bytes[16..16+data.len()].copy_from_slice(data);Ok(())
}
