use super::*;
use ephemeral_rollups_pinocchio::{instruction::delegate::delegate_account, types::DelegateConfig};

const CLOSING_TAG: u64 = u64::from_le_bytes(*b"IISCLOS1");
const SPONSOR_TAG: u64 = u64::from_le_bytes(*b"IISPONS1");
#[repr(C)]
struct Sponsor {
    tag: u64,
    admin: [u8; 32],
    accounts: u64,
    creations: u64,
}
fn load(data: &mut [u8]) -> core::result::Result<&mut Sponsor, ProgramError> {
    if data.len() != core::mem::size_of::<Sponsor>() || data.as_ptr().align_offset(8) != 0 {
        return Err(invalid());
    }
    let s = unsafe { &mut *data.as_mut_ptr().cast::<Sponsor>() };
    if s.tag != SPONSOR_TAG && s.tag != CLOSING_TAG {
        return Err(invalid());
    }
    Ok(s)
}
fn validate_any(
    sponsor: &AccountView,
    admin: Option<&AccountView>,
) -> core::result::Result<([u8; 32], u8), ProgramError> {
    owned(sponsor)?;
    let mut bytes_view = *sponsor;
    let mut bytes = bytes_view.try_borrow_mut()?;
    let s = load(&mut bytes)?;
    if let Some(a) = admin {
        if !a.is_signer() || a.address().as_array() != &s.admin {
            return Err(ProgramError::MissingRequiredSignature);
        }
    }
    let bump = pda(sponsor, &[b"sponsor", &s.admin])?;
    Ok((s.admin, bump))
}
fn validate(
    sponsor: &AccountView,
    admin: Option<&AccountView>,
) -> core::result::Result<([u8; 32], u8), ProgramError> {
    let result = validate_any(sponsor, admin)?;
    if sponsor.try_borrow()?[..8] != SPONSOR_TAG.to_le_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(result)
}
fn accounts(sponsor: &AccountView, delta: i64) -> ProgramResult {
    let mut bytes_view = *sponsor;
    let mut bytes = bytes_view.try_borrow_mut()?;
    let s = load(&mut bytes)?;
    s.accounts = s.accounts.checked_add_signed(delta).ok_or(invalid())?;
    Ok(())
}
pub fn initialize(
    admin: &AccountView,
    sponsor: &AccountView,
    system: &AccountView,
    funding: u64,
) -> ProgramResult {
    if !admin.is_signer()
        || !admin.is_writable()
        || system.address() != &pinocchio_system::ID
        || !system.executable()
    {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let bump = pda(sponsor, &[b"sponsor", admin.address().as_ref()])?;
    if sponsor.data_len() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    let rent = minimum_rent(core::mem::size_of::<Sponsor>())?;
    let lamports = rent
        .checked_add(funding)
        .ok_or(ProgramError::InvalidArgument)?;
    let bump = [bump];
    let seeds = [
        Seed::from(b"sponsor"),
        Seed::from(admin.address().as_ref()),
        Seed::from(&bump),
    ];
    if !sponsor.owned_by(&pinocchio_system::ID) {
        return Err(invalid());
    }
    if sponsor.lamports() == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: admin,
            to: sponsor,
            lamports,
            space: core::mem::size_of::<Sponsor>() as u64,
            owner: &ID,
        }
        .invoke_signed(&[PdaSigner::from(&seeds)])?;
    } else {
        let shortfall = lamports.saturating_sub(sponsor.lamports());
        if shortfall > 0 {
            pinocchio_system::instructions::Transfer {
                from: admin,
                to: sponsor,
                lamports: shortfall,
            }
            .invoke()?;
        }
        pinocchio_system::instructions::Allocate {
            account: sponsor,
            space: core::mem::size_of::<Sponsor>() as u64,
        }
        .invoke_signed(&[PdaSigner::from(&seeds)])?;
        pinocchio_system::instructions::Assign {
            account: sponsor,
            owner: &ID,
        }
        .invoke_signed(&[PdaSigner::from(&seeds)])?;
    }
    let mut bytes_view = *sponsor;
    let mut bytes = bytes_view.try_borrow_mut()?;
    bytes[..8].copy_from_slice(&SPONSOR_TAG.to_le_bytes());
    bytes[8..40].copy_from_slice(admin.address().as_ref());
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn delegate(
    admin: &AccountView,
    sponsor: &AccountView,
    owner_program: &AccountView,
    buffer: &AccountView,
    record: &AccountView,
    metadata: &AccountView,
    delegation: &AccountView,
    system: &AccountView,
    validator: [u8; 32],
) -> ProgramResult {
    let (owner, bump) = validate(sponsor, Some(admin))?;
    if owner_program.address() != &ID
        || !owner_program.executable()
        || delegation.address() != &DELEGATION_PROGRAM_ID
        || !delegation.executable()
        || system.address() != &pinocchio_system::ID
    {
        return Err(ProgramError::IncorrectProgramId);
    }
    let mut views = [
        *admin,
        *sponsor,
        *owner_program,
        *buffer,
        *record,
        *metadata,
        *system,
    ];
    delegate_account(
        &mut views,
        &[b"sponsor", &owner],
        bump,
        DelegateConfig {
            commit_frequency_ms: u32::MAX,
            validator: Some(Address::new_from_array(validator)),
        },
    )
}
pub fn prepare_topology(
    admin: &AccountView,
    sponsor: &AccountView,
    top: &AccountView,
    rent_vault: &AccountView,
    program: &AccountView,
) -> ProgramResult {
    vault(rent_vault, program)?;
    let (owner, bump) = validate(sponsor, Some(admin))?;
    let resolution = RESOLUTION.to_le_bytes();
    let top_bump = [pda(top, &[b"topology", &resolution])?];
    let bump = [bump];
    let ss = [
        Seed::from(b"sponsor"),
        Seed::from(&owner),
        Seed::from(&bump),
    ];
    let ts = [
        Seed::from(b"topology"),
        Seed::from(&resolution),
        Seed::from(&top_bump),
    ];
    let signers = [PdaSigner::from(&ss), PdaSigner::from(&ts)];
    let old = top.data_len();
    let total = 32 + TOPOLOGY.len();
    if old == total {
        return Ok(());
    }
    if old > total {
        return Err(invalid());
    }
    let end = (old + 8192).min(total);
    let eph = EphemeralAccount::new(sponsor, top, rent_vault, program).with_signers(&signers);
    if old == 0 {
        eph.create(end as u32)?;
        accounts(sponsor, 1)?;
    } else {
        owned(top)?;
        if top.try_borrow()?[..32] != *sponsor.address().as_array() {
            return Err(invalid());
        }
        eph.resize(end as u32)?;
    }
    let mut bytes_view = *top;
    let mut bytes = bytes_view.try_borrow_mut()?;
    if old == 0 {
        bytes[..32].copy_from_slice(sponsor.address().as_ref());
    }
    let start = old.max(32);
    bytes[start..end].copy_from_slice(&TOPOLOGY[start - 32..end - 32]);
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn create(
    owner: &AccountView,
    admin: &AccountView,
    sponsor: &AccountView,
    planet: &AccountView,
    engine: &AccountView,
    rent_vault: &AccountView,
    program: &AccountView,
    nonce: u64,
    seed: u32,
    session: [u8; 32],
    session_signer: Option<&AccountView>,
) -> ProgramResult {
    if nonce != 0 || session == [0; 32] || session == *owner.address().as_array() {
        return Err(ProgramError::InvalidArgument);
    }
    vault(rent_vault, program)?;
    if !owner.is_signer() && !session_signer.is_some_and(|key| key.is_signer() && key.address().as_array() == &session) {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let (sowner, sbump) = validate(sponsor, Some(admin))?;
    let nonce_bytes = nonce.to_le_bytes();
    let pbump = [pda(
        planet,
        &[b"planet", owner.address().as_ref(), &nonce_bytes],
    )?];
    let ebump = [pda(engine, &[b"engine", planet.address().as_ref()])?];
    if planet.data_len() != 0 || engine.data_len() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    let sbump = [sbump];
    let ss = [
        Seed::from(b"sponsor"),
        Seed::from(&sowner),
        Seed::from(&sbump),
    ];
    let ps = [
        Seed::from(b"planet"),
        Seed::from(owner.address().as_ref()),
        Seed::from(&nonce_bytes),
        Seed::from(&pbump),
    ];
    let es = [
        Seed::from(b"engine"),
        Seed::from(planet.address().as_ref()),
        Seed::from(&ebump),
    ];
    EphemeralAccount::new(sponsor, planet, rent_vault, program)
        .with_signers(&[PdaSigner::from(&ss), PdaSigner::from(&ps)])
        .create(core::mem::size_of::<Planet>() as u32)?;
    EphemeralAccount::new(sponsor, engine, rent_vault, program)
        .with_signers(&[PdaSigner::from(&ss), PdaSigner::from(&es)])
        .create((WORDS * 8) as u32)?;
    let mut pbytes_view = *planet;
    let mut pbytes = pbytes_view.try_borrow_mut()?;
    let mut ebytes_view = *engine;
    let mut ebytes = ebytes_view.try_borrow_mut()?;
    let p = unsafe { &mut *pbytes.as_mut_ptr().cast::<Planet>() };
    p.initialize(
        load_engine(&mut ebytes)?,
        *owner.address().as_array(),
        *sponsor.address().as_array(),
        nonce,
        seed,
        now()?,
    );
    p.session = session;
    {
        let mut view = *sponsor;
        let mut bytes = view.try_borrow_mut()?;
        let sponsor_state = load(&mut bytes)?;
        sponsor_state.creations = sponsor_state.creations.checked_add(1).ok_or(invalid())?;
        let digest = solana_sha256_hasher::hashv(&[
            planet.address().as_ref(),
            &Clock::get()?.slot.to_le_bytes(),
            &sponsor_state.creations.to_le_bytes(),
        ])
        .to_bytes();
        p.revision = u64::from_le_bytes(digest[..8].try_into().unwrap()) & (u64::MAX >> 1);
    }
    accounts(sponsor, 2)
}
#[allow(clippy::too_many_arguments)]
pub fn close(
    owner: &AccountView,
    sponsor: &AccountView,
    planet: &AccountView,
    engine: &AccountView,
    rent_vault: &AccountView,
    context: &AccountView,
    program: &AccountView,
) -> ProgramResult {
    vault(rent_vault, program)?;
    magic(program, context)?;
    let (sowner, sbump) = validate(sponsor, None)?;
    {
        let mut bytes_view = *planet;
        let mut bytes = bytes_view.try_borrow_mut()?;
        let p = load_planet(&mut bytes)?;
        validate_pair(planet, engine, p)?;
        if !owner.is_signer()
            || (owner.address().as_array() != &p.owner && owner.address().as_array() != &p.session)
            || sponsor.address().as_array() != &p.sponsor
        {
            return Err(ProgramError::MissingRequiredSignature);
        }
    }
    scheduling::cancel(planet, context, program)?;
    let sbump = [sbump];
    let ss = [
        Seed::from(b"sponsor"),
        Seed::from(&sowner),
        Seed::from(&sbump),
    ];
    let signers = [PdaSigner::from(&ss)];
    EphemeralAccount::new(sponsor, engine, rent_vault, program)
        .with_signers(&signers)
        .close()?;
    EphemeralAccount::new(sponsor, planet, rent_vault, program)
        .with_signers(&signers)
        .close()?;
    accounts(sponsor, -2)
}
pub fn close_topology(
    admin: &AccountView,
    sponsor: &AccountView,
    top: &AccountView,
    rent_vault: &AccountView,
    program: &AccountView,
) -> ProgramResult {
    vault(rent_vault, program)?;
    let (owner, bump) = validate(sponsor, Some(admin))?;
    pda(top, &[b"topology", &RESOLUTION.to_le_bytes()])?;
    owned(top)?;
    {
        let mut view = *sponsor;
        let mut bytes = view.try_borrow_mut()?;
        if load(&mut bytes)?.accounts != 1 {
            return Err(ProgramError::Custom(107));
        }
    }
    if top.try_borrow()?[..32] != *sponsor.address().as_array() {
        return Err(invalid());
    }
    let bump = [bump];
    let seeds = [
        Seed::from(b"sponsor"),
        Seed::from(&owner),
        Seed::from(&bump),
    ];
    EphemeralAccount::new(sponsor, top, rent_vault, program)
        .with_signers(&[PdaSigner::from(&seeds)])
        .close()?;
    accounts(sponsor, -1)
}
pub fn request_undelegate(
    admin: &AccountView,
    sponsor: &AccountView,
    context: &AccountView,
    program: &AccountView,
) -> ProgramResult {
    magic(program, context)?;
    validate(sponsor, Some(admin))?;
    {
        let mut view = *sponsor;
        let mut bytes = view.try_borrow_mut()?;
        let state = load(&mut bytes)?;
        if state.accounts != 0 {
            return Err(ProgramError::Custom(107));
        }
        state.tag = CLOSING_TAG;
    }
    let accounts = [*sponsor];
    let mut buffer = [0u8; 512];
    ephemeral_rollups_pinocchio::intent_bundle::MagicIntentBundleBuilder::new(
        *admin, *context, *program,
    )
    .commit_and_undelegate(&accounts)
    .build_and_invoke(&mut buffer)
}
pub fn undelegate(
    sponsor: &AccountView,
    buffer: &AccountView,
    payer: &AccountView,
    system: &AccountView,
    seeds: Vec<Vec<u8>>,
) -> ProgramResult {
    if system.address() != &pinocchio_system::ID
        || seeds.len() != 2
        || seeds[0] != b"sponsor"
        || seeds[1].len() != 32
    {
        return Err(invalid());
    }
    pda(sponsor, &[&seeds[0], &seeds[1]])?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&2u32.to_le_bytes());
    for seed in seeds {
        bytes.extend_from_slice(&(seed.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&seed);
    }
    let mut view = *sponsor;
    ephemeral_rollups_pinocchio::instruction::undelegate::undelegate(
        &mut view, &ID, buffer, payer, &bytes,
    )
}
pub fn reclaim(admin: &AccountView, sponsor: &AccountView) -> ProgramResult {
    validate_any(sponsor, Some(admin))?;
    {
        let mut view = *sponsor;
        let mut bytes = view.try_borrow_mut()?;
        if load(&mut bytes)?.accounts != 0 {
            return Err(ProgramError::Custom(107));
        }
        bytes.fill(0);
    }
    let balance = admin
        .lamports()
        .checked_add(sponsor.lamports())
        .ok_or(invalid())?;
    let mut a = *admin;
    let mut s = *sponsor;
    a.set_lamports(balance);
    s.set_lamports(0);
    Ok(())
}

fn minimum_rent(size: usize) -> core::result::Result<u64, ProgramError> {
    let mut bytes = [0u8; 16];
    let id = pinocchio::sysvars::rent::RENT_ID;
    let legacy = pinocchio::sysvars::get_sysvar(&mut bytes, &id, 0).is_ok();
    if !legacy {
        pinocchio::sysvars::get_sysvar(&mut bytes[..8], &id, 0)?;
    }
    let rate = u64::from_le_bytes(bytes[..8].try_into().unwrap());
    let threshold = u64::from_le_bytes(bytes[8..].try_into().unwrap());
    let factor = match threshold {
        0 | 0x3ff0000000000000 => 1,
        0x4000000000000000 => 2,
        _ => return Err(invalid()),
    };
    rate.checked_mul(factor)
        .and_then(|r| r.checked_mul(size as u64 + 128))
        .ok_or(invalid())
}

pub fn link_session(admin: &AccountView, sponsor: &AccountView, planet: &AccountView, session: &AccountView) -> ProgramResult {
    validate(sponsor, Some(admin))?;
    owned(planet)?;
    let mut view = *planet;
    let mut bytes = view.try_borrow_mut()?;
    if bytes.len()!=core::mem::offset_of!(Planet,needs)&&bytes.len()!=core::mem::size_of::<Planet>(){return Err(invalid());}
    if bytes[..8]!=PLANET_TAG.to_le_bytes()&&bytes[..8]!=LEGACY_TAG.to_le_bytes(){return Err(invalid());}
    let owner:[u8;32]=bytes[8..40].try_into().map_err(|_|invalid())?;
    pda(planet,&[b"planet",&owner,&bytes[72..80]])?;
    if !session.is_signer()||session.address().as_array()==&owner||session.address().as_array()==&[0;32]||bytes[40..72]!=*sponsor.address().as_array(){return Err(ProgramError::MissingRequiredSignature);}
    bytes[6288..6320].copy_from_slice(session.address().as_ref());
    Ok(())
}

pub fn migrate_planet(authority:&AccountView,sponsor:&AccountView,planet:&AccountView,rent_vault:&AccountView,program:&AccountView)->ProgramResult {
    vault(rent_vault,program)?;
    owned(planet)?;
    let (sowner,sbump)=validate(sponsor,None)?;
    let old=core::mem::offset_of!(Planet,needs);
    let total=core::mem::size_of::<Planet>();
    let (owner,nonce,pbump)={
        let bytes=planet.try_borrow()?;
        if bytes.len()!=old&&bytes.len()!=total{return Err(invalid());}
        if (bytes[..8]!=PLANET_TAG.to_le_bytes()&&bytes[..8]!=LEGACY_TAG.to_le_bytes())||bytes[40..72]!=*sponsor.address().as_array(){return Err(invalid());}
        let owner:[u8;32]=bytes[8..40].try_into().map_err(|_|invalid())?;
        let session:[u8;32]=bytes[6288..6320].try_into().map_err(|_|invalid())?;
        if !authority.is_signer()||(authority.address().as_array()!=&owner&&authority.address().as_array()!=&session){return Err(ProgramError::MissingRequiredSignature);}
        let nonce:[u8;8]=bytes[72..80].try_into().map_err(|_|invalid())?;
        let bump=pda(planet,&[b"planet",&owner,&nonce])?;
        (owner,nonce,[bump])
    };
    if planet.data_len()==total{return Ok(());}
    let sbump=[sbump];
    let ss=[Seed::from(b"sponsor"),Seed::from(&sowner),Seed::from(&sbump)];
    let ps=[Seed::from(b"planet"),Seed::from(&owner),Seed::from(&nonce),Seed::from(&pbump)];
    EphemeralAccount::new(sponsor,planet,rent_vault,program).with_signers(&[PdaSigner::from(&ss),PdaSigner::from(&ps)]).resize(total as u32)?;
    let mut view=*planet;
    view.try_borrow_mut()?[old..].fill(0);
    Ok(())
}
