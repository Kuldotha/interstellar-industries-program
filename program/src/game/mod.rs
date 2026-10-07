#![allow(deprecated)]
mod lifecycle;
mod scheduling;
mod state;
use crate::*;
use ephemeral_rollups_pinocchio::{consts::*, ephemeral_accounts::EphemeralAccount};
use pinocchio::{
    cpi::{Seed, Signer as PdaSigner},
    sysvars::{clock::Clock, Sysvar},
    AccountView,
};
use solarium::prelude::*;
use solarium_program::prelude::*;
use solarium_program::{Account, Signer};
use state::*;
const SESSION_AUTHORITY: Address = Address::from_str_const("7SMQSQ6hhyy1xwPsEoFNWdnGxfR3BYdb4VPkjUrzHU2s");
pub fn now() -> core::result::Result<u64, ProgramError> {
    u64::try_from(Clock::get()?.unix_timestamp).map_err(|_| invalid())
}
pub fn pda(account: &AccountView, seeds: &[&[u8]]) -> core::result::Result<u8, ProgramError> {
    let (address, bump) = Address::find_program_address(seeds, &ID);
    if account.address() != &address {
        return Err(invalid());
    }
    Ok(bump)
}
pub fn owned(account: &AccountView) -> ProgramResult {
    if !account.owned_by(&ID) || !account.is_writable() {
        return Err(invalid());
    }
    Ok(())
}
pub fn magic(account: &AccountView, context: &AccountView) -> ProgramResult {
    if account.address() != &MAGIC_PROGRAM_ID
        || !account.executable()
        || context.address() != &MAGIC_CONTEXT_ID
        || !context.is_writable()
    {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
pub fn vault(account: &AccountView, program: &AccountView) -> ProgramResult {
    if account.address() != &EPHEMERAL_VAULT_ID
        || !account.is_writable()
        || program.address() != &MAGIC_PROGRAM_ID
        || !program.executable()
    {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
pub fn topology(
    account: &AccountView,
) -> core::result::Result<pinocchio::account::Ref<'_, [u8]>, ProgramError> {
    pda(account, &[b"topology", &RESOLUTION.to_le_bytes()])?;
    if !account.owned_by(&ID) || account.is_writable() {
        return Err(invalid());
    }
    let data = account.try_borrow()?;
    if data.len() != 32 + TOPOLOGY.len() {
        return Err(invalid());
    }
    Ok(data)
}
pub fn load_planet(data: &mut [u8]) -> core::result::Result<&mut Planet, ProgramError> {
    if data.len() != core::mem::size_of::<Planet>() || data.as_ptr().align_offset(8) != 0 {
        return Err(invalid());
    }
    let p = unsafe { &mut *(data.as_mut_ptr().cast::<Planet>()) };
    if (p.tag != PLANET_TAG && p.tag != LEGACY_TAG) || p.resolution != RESOLUTION {
        return Err(invalid());
    }
    Ok(p)
}
pub fn load_engine(data: &mut [u8]) -> core::result::Result<&mut [u64; WORDS], ProgramError> {
    if data.len() != WORDS * 8 || data.as_ptr().align_offset(8) != 0 {
        return Err(invalid());
    }
    Ok(unsafe { &mut *(data.as_mut_ptr().cast::<[u64; WORDS]>()) })
}
fn validate_pair(planet: &AccountView, engine: &AccountView, p: &Planet) -> ProgramResult {
    owned(planet)?;
    owned(engine)?;
    pda(planet, &[b"planet", &p.owner, &p.nonce.to_le_bytes()])?;
    pda(engine, &[b"engine", planet.address().as_ref()])?;
    Ok(())
}
fn action(
    authority: &AccountView,
    planet: &AccountView,
    engine: &AccountView,
    top: &AccountView,
    context: &AccountView,
    program: &AccountView,
    kind: u8,
    tile: u32,
    value: u8,
    facing: u8,
) -> ProgramResult {
    magic(program, context)?;
    let t = topology(top)?;
    let mut pbytes_view = *planet;
    let mut pbytes = pbytes_view.try_borrow_mut()?;
    let p = load_planet(&mut pbytes)?;
    validate_pair(planet, engine, p)?;
    if !authority.is_signer() || (authority.address().as_array() != &p.owner && authority.address().as_array() != &p.session) {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let mut ebytes_view = *engine;
    let mut ebytes = ebytes_view.try_borrow_mut()?;
    let e = load_engine(&mut ebytes)?;
    if p.tag!=PLANET_TAG{return Err(ProgramError::Custom(109));}
    let timestamp = now()?;
    p.advance_for_action(e, timestamp)?;
    match kind {
        0 => p.build(e, &t[32..], tile as usize, value, facing as usize)?,
        1 => p.demolish(e, &t[32..], tile as usize)?,
        2 => p.pause(e, &t[32..], tile as usize, value)?,
        _ => return Err(ProgramError::InvalidInstructionData),
    }
    p.refresh(e)?;
    drop(ebytes);
    drop(pbytes);
    scheduling::sync(planet, engine, context, program, timestamp, false)
}
fn advance(
    planet: &AccountView,
    engine: &AccountView,
    context: &AccountView,
    program: &AccountView,
    expected: Option<u64>,
) -> ProgramResult {
    magic(program, context)?;
    if planet.data_len() == 0 && expected.is_some() {
        return Ok(());
    }
    let mut pbytes_view = *planet;
    let mut pbytes = pbytes_view.try_borrow_mut()?;
    let p = load_planet(&mut pbytes)?;
    validate_pair(planet, engine, p)?;
    if p.tag!=PLANET_TAG{return Ok(());}
    let timestamp = now()?;
    if let Some(revision) = expected {
        if revision != p.revision || timestamp < p.due {
            return Ok(());
        }
    }
    let mut ebytes_view = *engine;
    let mut ebytes = ebytes_view.try_borrow_mut()?;
    p.advance(load_engine(&mut ebytes)?, timestamp)?;
    drop(ebytes);
    drop(pbytes);
    scheduling::sync(
        planet,
        engine,
        context,
        program,
        timestamp,
        expected.is_some(),
    )
}
#[program(id = "4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd")]
impl Interstellar {
    #[instruction(discriminator = 31)]
    pub fn update_rules<'a>(&self,authority:&Signer<'a>,planet:&mut Account<'a>,engine:&mut Account<'a>,context:&mut Account<'a>,magic_program:&Account<'a>)->Result<()>{
        let planet=planet.info.as_view();let engine=engine.info.as_view();let context=context.info.as_view();let program=magic_program.info.as_view();
        magic(program,context)?;
        {let mut pv=*planet;let mut pb=pv.try_borrow_mut()?;let p=load_planet(&mut pb)?;validate_pair(planet,engine,p)?;
        let signer=authority.info.as_view().address().as_array();if signer!=&p.owner&&signer!=&p.session{return Err(ProgramError::MissingRequiredSignature.into());}
        if p.tag==PLANET_TAG{return Ok(());}
        let mut ev=*engine;let mut eb=ev.try_borrow_mut()?;let e=load_engine(&mut eb)?;
        if e[READY]!=0{colony::advance_with(e,&mut p.clock,core::slice::from_mut(&mut p.tier),&p.needs,now()?).map_err(|_|invalid())?;}
        e[8]=0;e[10]=0;e[12]=0;p.tag=PLANET_TAG;p.clock.dirty=1;p.refresh(e)?;}
        scheduling::sync(planet,engine,context,program,now()?,false)?;Ok(())
    }

    #[instruction(discriminator = 30)]
    pub fn migrate_planet<'a>(&self,authority:&Signer<'a>,sponsor:&mut Account<'a>,planet:&mut Account<'a>,vault:&mut Account<'a>,magic:&Account<'a>)->Result<()>{
        Ok(lifecycle::migrate_planet(authority.info.as_view(),sponsor.info.as_view(),planet.info.as_view(),vault.info.as_view(),magic.info.as_view())?)
    }

    #[instruction(discriminator = 10)]
    pub fn initialize<'a>(
        &self,
        admin: &Signer<'a>,
        sponsor: &mut Account<'a>,
        system: &Account<'a>,
        funding: u64,
    ) -> Result<()> {
        Ok(lifecycle::initialize(
            admin.info.as_view(),
            sponsor.info.as_view(),
            system.info.as_view(),
            funding,
        )?)
    }
    #[instruction(discriminator = 11)]
    pub fn delegate_sponsor<'a>(
        &self,
        admin: &Signer<'a>,
        sponsor: &mut Account<'a>,
        owner_program: &Account<'a>,
        buffer: &mut Account<'a>,
        record: &mut Account<'a>,
        metadata: &mut Account<'a>,
        delegation: &Account<'a>,
        system: &Account<'a>,
        validator: [u8; 32],
    ) -> Result<()> {
        Ok(lifecycle::delegate(
            admin.info.as_view(),
            sponsor.info.as_view(),
            owner_program.info.as_view(),
            buffer.info.as_view(),
            record.info.as_view(),
            metadata.info.as_view(),
            delegation.info.as_view(),
            system.info.as_view(),
            validator,
        )?)
    }
    #[instruction(discriminator = 12)]
    pub fn prepare_topology<'a>(
        &self,
        admin: &Signer<'a>,
        sponsor: &mut Account<'a>,
        topology: &mut Account<'a>,
        vault: &mut Account<'a>,
        magic: &Account<'a>,
    ) -> Result<()> {
        Ok(lifecycle::prepare_topology(
            admin.info.as_view(),
            sponsor.info.as_view(),
            topology.info.as_view(),
            vault.info.as_view(),
            magic.info.as_view(),
        )?)
    }
    #[instruction(discriminator = 20)]
    pub fn create_planet<'a>(
        &self,
        owner: &Signer<'a>,
        admin: &Signer<'a>,
        sponsor: &mut Account<'a>,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        vault: &mut Account<'a>,
        magic: &Account<'a>,
        nonce: u64,
        seed: u32,
        session: [u8; 32],
    ) -> Result<()> {
        Ok(lifecycle::create(
            owner.info.as_view(),
            admin.info.as_view(),
            sponsor.info.as_view(),
            planet.info.as_view(),
            engine.info.as_view(),
            vault.info.as_view(),
            magic.info.as_view(),
            nonce,
            seed,
            session,
            None,
        )?)
    }
    #[instruction(discriminator = 28)]
    pub fn create_with_session<'a>(
        &self, owner: &Account<'a>, admin: &Signer<'a>, session: &Signer<'a>,
        sponsor: &mut Account<'a>, planet: &mut Account<'a>, engine: &mut Account<'a>,
        vault: &mut Account<'a>, magic: &Account<'a>, nonce: u64, seed: u32,
    ) -> Result<()> {
        if admin.info.as_view().address() != &SESSION_AUTHORITY { return Err(ProgramError::MissingRequiredSignature.into()); }
        Ok(lifecycle::create(owner.info.as_view(), admin.info.as_view(), sponsor.info.as_view(),
            planet.info.as_view(), engine.info.as_view(), vault.info.as_view(), magic.info.as_view(),
            nonce, seed, *session.info.as_view().address().as_array(), Some(session.info.as_view()))?)
    }
    #[instruction(discriminator = 29)]
    pub fn link_session<'a>(&self, admin: &Signer<'a>, session: &Signer<'a>, sponsor: &mut Account<'a>, planet: &mut Account<'a>) -> Result<()> {
        if admin.info.as_view().address() != &SESSION_AUTHORITY { return Err(ProgramError::MissingRequiredSignature.into()); }
        Ok(lifecycle::link_session(admin.info.as_view(), sponsor.info.as_view(), planet.info.as_view(), session.info.as_view())?)
    }
    #[instruction(discriminator = 27)]
    pub fn authorize_session<'a>(&self, owner: &Signer<'a>, planet: &mut Account<'a>, session: [u8; 32]) -> Result<()> {
        let account = planet.info.as_view();
        owned(account)?;
        let mut view = *account;
        let mut bytes = view.try_borrow_mut()?;
        let p = load_planet(&mut bytes)?;
        pda(account, &[b"planet", &p.owner, &p.nonce.to_le_bytes()])?;
        if owner.info.as_view().address().as_array() != &p.owner || session == [0; 32] || session == p.owner {
            return Err(ProgramError::MissingRequiredSignature.into());
        }
        p.session = session;
        Ok(())
    }
    #[instruction(discriminator = 21)]
    pub fn build<'a>(
        &self,
        owner: &Signer<'a>,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        topology: &Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
        tile: u32,
        kind: u8,
        facing: u8,
    ) -> Result<()> {
        Ok(action(
            owner.info.as_view(),
            planet.info.as_view(),
            engine.info.as_view(),
            topology.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
            0,
            tile,
            kind,
            facing,
        )?)
    }
    #[instruction(discriminator = 22)]
    pub fn demolish<'a>(
        &self,
        owner: &Signer<'a>,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        topology: &Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
        tile: u32,
    ) -> Result<()> {
        Ok(action(
            owner.info.as_view(),
            planet.info.as_view(),
            engine.info.as_view(),
            topology.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
            1,
            tile,
            0,
            0,
        )?)
    }
    #[instruction(discriminator = 23)]
    pub fn set_paused<'a>(
        &self,
        owner: &Signer<'a>,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        topology: &Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
        tile: u32,
        paused: u8,
    ) -> Result<()> {
        Ok(action(
            owner.info.as_view(),
            planet.info.as_view(),
            engine.info.as_view(),
            topology.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
            2,
            tile,
            paused,
            0,
        )?)
    }
    #[instruction(discriminator = 24)]
    pub fn advance<'a>(
        &self,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
    ) -> Result<()> {
        Ok(advance(
            planet.info.as_view(),
            engine.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
            None,
        )?)
    }
    #[instruction(discriminator = 25)]
    pub fn scheduled_advance<'a>(
        &self,
        signer: &Signer<'a>,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
        revision: u64,
    ) -> Result<()> {
        if signer.info.as_view().address() != &scheduling::signer(planet.info.as_view().address()) {
            return Err(ProgramError::MissingRequiredSignature.into());
        }
        Ok(advance(
            planet.info.as_view(),
            engine.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
            Some(revision),
        )?)
    }
    #[instruction(discriminator = 26)]
    pub fn close_planet<'a>(
        &self,
        owner: &Signer<'a>,
        sponsor: &mut Account<'a>,
        planet: &mut Account<'a>,
        engine: &mut Account<'a>,
        vault: &mut Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
    ) -> Result<()> {
        Ok(lifecycle::close(
            owner.info.as_view(),
            sponsor.info.as_view(),
            planet.info.as_view(),
            engine.info.as_view(),
            vault.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
        )?)
    }
    #[instruction(discriminator = 13)]
    pub fn close_topology<'a>(
        &self,
        admin: &Signer<'a>,
        sponsor: &mut Account<'a>,
        topology: &mut Account<'a>,
        vault: &mut Account<'a>,
        magic: &Account<'a>,
    ) -> Result<()> {
        Ok(lifecycle::close_topology(
            admin.info.as_view(),
            sponsor.info.as_view(),
            topology.info.as_view(),
            vault.info.as_view(),
            magic.info.as_view(),
        )?)
    }
    #[instruction(discriminator = 14)]
    pub fn request_undelegation<'a>(
        &self,
        admin: &Signer<'a>,
        sponsor: &mut Account<'a>,
        context: &mut Account<'a>,
        magic: &Account<'a>,
    ) -> Result<()> {
        Ok(lifecycle::request_undelegate(
            admin.info.as_view(),
            sponsor.info.as_view(),
            context.info.as_view(),
            magic.info.as_view(),
        )?)
    }
    #[instruction(discriminator = 15, alias = "global:process_undelegation")]
    pub fn process_undelegation<'a>(
        &self,
        sponsor: &mut Account<'a>,
        buffer: &mut Account<'a>,
        payer: &mut Account<'a>,
        system: &Account<'a>,
        seeds: Vec<Vec<u8>>,
    ) -> Result<()> {
        Ok(lifecycle::undelegate(
            sponsor.info.as_view(),
            buffer.info.as_view(),
            payer.info.as_view(),
            system.info.as_view(),
            seeds,
        )?)
    }
    #[instruction(discriminator = 16)]
    pub fn reclaim_sponsor<'a>(&self, admin: &Signer<'a>, sponsor: &mut Account<'a>) -> Result<()> {
        Ok(lifecycle::reclaim(
            admin.info.as_view(),
            sponsor.info.as_view(),
        )?)
    }
}
