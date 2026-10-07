pub mod api {
    use solarium::prelude::*;
    use solarium_program::prelude::*;
    use solarium_program::Account;

    #[program(id = "4BBtj9tUo1xeZFkrW3LBg9GrUTtTezGYGBKqjs7A7fAE")]
    impl ProductionLab {
        #[instruction(discriminator = 1)]
        pub fn advance<'a>(&self, planet: &mut Account<'a>, initialize: u8) -> Result<()> {
            Ok(super::advance_planet(planet,initialize)?)
        }
        #[instruction(discriminator = 0)]
        pub fn update<'a>(&self, planet: &mut Account<'a>, action: u8) -> Result<()> {
            Ok(super::update_planet(planet, action)?)
        }
    }
}
extern "C"{fn sol_remaining_compute_units()->u64;fn sol_set_return_data(p:*const u8,n:u64);}
fn update_planet(planet:&mut solarium_program::Account<'_>,action:u8)->ProgramResult {
    let view=planet.info.as_view();
    if !view.owned_by(&Address::new_from_array([47;32])) || !view.is_writable(){return Err(ProgramError::Custom(1));}
    let mut data=planet.info.try_borrow_mut_data()?;
    if data.len()!=WORDS*8 || data.as_ptr().align_offset(8)!=0{return Err(ProgramError::Custom(1));}
    let state=unsafe{&mut *(data.as_mut_ptr() as *mut [u64;WORDS])};
    let before=unsafe{sol_remaining_compute_units()};
    let action=action as usize;
    let stamp=profile::start();
    prepare_stock(state).map_err(|_|ProgramError::Custom(4))?;
    profile::end(0,stamp);let stamp=profile::start();
    let change=prepare(state,action).map_err(|_|ProgramError::Custom(2))?;
    profile::end(1,stamp);
    let updated=unsafe{sol_remaining_compute_units()};
    execute(state,change).map_err(|_|ProgramError::Custom(3))?;
    let after=unsafe{sol_remaining_compute_units()};
    let costs=[before-updated,updated-after];
    #[cfg(not(feature="cu-profile"))]
    unsafe{sol_set_return_data(costs.as_ptr().cast(),16)};
    #[cfg(feature="cu-profile")]
    {let mut report=[0u64;15];report[..2].copy_from_slice(&costs);report[2..].copy_from_slice(&profile::totals());unsafe{sol_set_return_data(report.as_ptr().cast(),120)};}
    Ok(())
}

fn advance_planet(planet:&mut solarium_program::Account<'_>,initialize:u8)->ProgramResult {
    use pinocchio::sysvars::{clock::Clock,Sysvar};
    let now=Clock::get()?.unix_timestamp;if now<0{return Err(ProgramError::Custom(5));}let target=now as u64;
    let view=planet.info.as_view();
    if !view.owned_by(&Address::new_from_array([47;32]))||!view.is_writable(){return Err(ProgramError::Custom(1));}
    let mut data=planet.info.try_borrow_mut_data()?;
    let header=(WORDS+colony::CLOCK_WORDS)*8;
    if data.len()<header||(data.len()-header)%(colony::TIER_WORDS*8)!=0||data.as_ptr().align_offset(8)!=0{return Err(ProgramError::Custom(1));}
    let count=(data.len()-header)/(colony::TIER_WORDS*8);if count>8{return Err(ProgramError::Custom(1));}
    let (engine,rest)=data.split_at_mut(WORDS*8);let (clock,tiers)=rest.split_at_mut(colony::CLOCK_WORDS*8);
    let state=unsafe{&mut *(engine.as_mut_ptr() as *mut [u64;WORDS])};
    let clock=unsafe{&mut *(clock.as_mut_ptr() as *mut colony::ClockState)};
    let tiers=unsafe{core::slice::from_raw_parts_mut(tiers.as_mut_ptr() as *mut population::PopulationTier,count)};
    if initialize>1{return Err(ProgramError::Custom(5));}
    if initialize==1&&state[READY]==0&&clock.solves==0{clock.tick=target;}
    let before=unsafe{sol_remaining_compute_units()};
    if initialize==0{colony::advance(state,clock,tiers,target).map_err(|_|ProgramError::Custom(5))?;}
    let advanced=unsafe{sol_remaining_compute_units()};
    colony::refresh(state,clock,tiers).map_err(|_|ProgramError::Custom(6))?;
    let costs=[before-advanced,advanced-unsafe{sol_remaining_compute_units()}];
    unsafe{sol_set_return_data(costs.as_ptr().cast(),16)};Ok(())
}
