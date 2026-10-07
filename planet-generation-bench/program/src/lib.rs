#![no_std]
include!("../../common.rs");
use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};
pinocchio::program_entrypoint!(process, 2);
pinocchio::nostd_panic_handler!();
pinocchio::no_allocator!();
fn process(program: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    if accounts.len() != 2 || data.len() != 13 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (top, rest) = accounts.split_at_mut(1);
    if top[0].is_writable()
        || !top[0].owned_by(program)
        || !rest[0].is_writable()
        || !rest[0].owned_by(program)
    {
        return Err(ProgramError::InvalidAccountData);
    }
    let t = top[0].try_borrow()?;
    if t.len() < 8 {
        return Err(ProgramError::InvalidAccountData);
    }
    let n = tiles(&t);
    if t.len() != 8 + n * 36 {
        return Err(ProgramError::InvalidAccountData);
    }
    let mut state = rest[0].try_borrow_mut()?;
    if (7..=9).contains(&data[0]) {
        if state.len() != on_demand::STATE_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }
        let id = u32::from_le_bytes(data[1..5].try_into().unwrap()) as usize;
        let side = u32::from_le_bytes(data[5..9].try_into().unwrap()) as usize;
        let seed = u32::from_le_bytes(data[9..13].try_into().unwrap());
        if data[0] == 9 {
            if state[4] != 0 {
                return Err(ProgramError::InvalidAccountData);
            }
            on_demand::initialize(&mut state, seed);
        } else {
            if state[4] != on_demand::VERSION
                || state[..4] != seed.to_le_bytes()
                || id >= n
                || (data[0] == 8 && side >= 6)
            {
                return Err(ProgramError::InvalidInstructionData);
            }
            on_demand::query(
                &t,
                &mut state,
                id,
                if data[0] == 8 { Some(side) } else { None },
            );
        }
        return Ok(());
    }
    if state.len() != state_size(n) || state.as_ptr().align_offset(4) != 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    let start = u32::from_le_bytes(data[1..5].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(data[5..9].try_into().unwrap()) as usize;
    let seed = u32::from_le_bytes(data[9..13].try_into().unwrap());
    if start > n || count > n - start {
        return Err(ProgramError::InvalidInstructionData);
    }
    match data[0] {
        0..=2 | 4 => terrain(&t, &mut state, start, count, data[0], seed),
        5 => {
            if state[4] != 0 {
                return Err(ProgramError::InvalidAccountData);
            }
            initialize_permutation(&mut state, n, seed);
        }
        6 => {
            if state[4] != 1 || state[..4] != seed.to_le_bytes() {
                return Err(ProgramError::InvalidAccountData);
            }
            cached_terrain(&t, &mut state, start, count);
        }
        3 => forests(&t, &mut state, seed),
        _ => return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}
