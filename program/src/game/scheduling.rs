use super::*;
use ephemeral_rollups_pinocchio::crank::{
    CancelCrankCpi, CrankInstruction, ScheduleCrankArgs, ScheduleCrankCpi,
};
use pinocchio::instruction::InstructionAccount;
const CRANK: Address = Address::from_str_const("Crank11111111111111111111111111111111111111");
pub fn signer(planet: &Address) -> Address {
    Address::find_program_address(&[b"crank-executor", planet.as_ref()], &CRANK).0
}
pub fn cancel(planet: &AccountView, context: &AccountView, program: &AccountView) -> ProgramResult {
    let mut bytes_view = *planet;
    let mut bytes = bytes_view.try_borrow_mut()?;
    let p = load_planet(&mut bytes)?;
    if p.due == NEVER {
        return Ok(());
    }
    let id = p.task_id;
    let owner = p.owner;
    let nonce = p.nonce.to_le_bytes();
    let bump = [pda(planet, &[b"planet", &owner, &nonce])?];
    p.due = NEVER;
    p.revision = p.revision.checked_add(1).ok_or(invalid())?;
    drop(bytes);
    let seeds = [
        Seed::from(b"planet"),
        Seed::from(&owner),
        Seed::from(&nonce),
        Seed::from(&bump),
    ];
    CancelCrankCpi {
        authority: *planet,
        task_context: *context,
        magic_program: *program,
        crank_id: id as i64,
    }
    .invoke_signed(&[PdaSigner::from(&seeds)])
}
pub fn sync(
    planet: &AccountView,
    engine: &AccountView,
    context: &AccountView,
    program: &AccountView,
    timestamp: u64,
    fired: bool,
) -> ProgramResult {
    let mut bytes_view = *planet;
    let mut bytes = bytes_view.try_borrow_mut()?;
    let p = load_planet(&mut bytes)?;
    let next = p.clock.next;
    if !fired && p.due == next {
        return Ok(());
    }
    let owner = p.owner;
    let nonce = p.nonce.to_le_bytes();
    let bump = [pda(planet, &[b"planet", &owner, &nonce])?];
    let old = if p.due == NEVER {
        None
    } else {
        Some(p.task_id)
    };
    p.revision = p.revision.checked_add(1).ok_or(invalid())?;
    p.due = next;
    let revision = p.revision;
    p.task_id = u64::from_le_bytes(
        solana_sha256_hasher::hashv(&[
            ID.as_ref(),
            planet.address().as_ref(),
            &revision.to_le_bytes(),
        ])
        .to_bytes()[..8]
            .try_into()
            .unwrap(),
    );
    let task_id = p.task_id;
    drop(bytes);
    let seeds = [
        Seed::from(b"planet"),
        Seed::from(&owner),
        Seed::from(&nonce),
        Seed::from(&bump),
    ];
    let signers = [PdaSigner::from(&seeds)];
    if let Some(id) = old {
        CancelCrankCpi {
            authority: *planet,
            task_context: *context,
            magic_program: *program,
            crank_id: id as i64,
        }
        .invoke_signed(&signers)?;
    }
    if next == NEVER {
        return Ok(());
    }
    let executor = signer(planet.address());
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&25u64.to_le_bytes());
    data[8..].copy_from_slice(&revision.to_le_bytes());
    let metas = [
        InstructionAccount::readonly_signer(&executor),
        InstructionAccount::writable(planet.address()),
        InstructionAccount::writable(engine.address()),
        InstructionAccount::writable(context.address()),
        InstructionAccount::readonly(program.address()),
    ];
    let instructions = [CrankInstruction::new(ID, &metas, &data)];
    let delay = next
        .saturating_sub(timestamp)
        .max(1)
        .checked_mul(1000)
        .and_then(|n| i64::try_from(n).ok())
        .ok_or(invalid())?;
    // The validator also fires immediately; the callback rejects execution before its deadline.
    let args = ScheduleCrankArgs::new(task_id as i64, &instructions)
        .execution_interval_millis(delay)
        .iterations(2);
    let context_accounts = [*context];
    let cpi = ScheduleCrankCpi::new(*planet, *program, &context_accounts, args);
    let mut encoded = [0u8; 512];
    cpi.invoke_signed::<2>(&mut encoded, &signers)
}
