use crate::colony_needs::{self,ColonyNeeds};
use crate::{population::{PopulationTier,workforce_capacity,project_stock,stock_boundary},*};
pub const FISH_RESOURCE:usize=23;
#[repr(C)]
#[derive(Clone)]
pub struct ClockState {
    pub tick:u64,
    pub next:u64,
    pub workers:u64,
    pub population:u64,
    pub solves:u64,
    pub dirty:u64,
    pub carry:[i64;R],
}
impl Default for ClockState{fn default()->Self{Self{tick:0,next:0,workers:0,population:0,solves:0,dirty:1,carry:[0;R]}}}
pub const CLOCK_WORDS:usize=core::mem::size_of::<ClockState>()/8;
pub const TIER_WORDS:usize=core::mem::size_of::<PopulationTier>()/8;
pub fn food(state:&[u64;WORDS])->u64{
    let demand=state[CONSUMPTION+FISH_RESOURCE];
    if demand==0{Q}else{mul_div(state[CONSUMED+FISH_RESOURCE],Q,demand).min(Q)}
}
pub fn refresh(state:&mut [u64;WORDS],clock:&mut ClockState,tiers:&mut [PopulationTier])->Result<(),()>{refresh_with(state,clock,tiers,&mut ColonyNeeds::default())}
pub fn refresh_with(state:&mut [u64;WORDS],clock:&mut ClockState,tiers:&mut [PopulationTier],needs:&mut ColonyNeeds)->Result<(),()>{
    let mut population=0;
    for tier in tiers.iter(){if !tier.valid(){return Err(());}population+=tier.population;}
    if population>65536||clock.workers>65536{return Err(());}
    if clock.dirty!=0||clock.population!=population||state[READY]==0{
        let mut original=[0;N];original.copy_from_slice(&state[..N]);
        let power=colony_power::plan(original,population,clock.workers,needs.generators,needs.power_demand,state[stock_index(6)]);
        state[..N].copy_from_slice(&power.capacities);
        state[AVAILABLE]=workforce_capacity(power.capacities.iter().sum(),population,clock.workers);
        let demand=colony_needs::demands(population,clock.workers,needs.generators,needs.power_demand);
        for r in [23,79,5,6]{state[CONSUMPTION+r]=demand[r];}
        state[CONSUMPTION+6]=power.fuel;
        let solved=(||{prepare_stock(state)?;let change=prepare(state,255)?;execute(state,change)})();
        state[..N].copy_from_slice(&original);solved?;
        clock.population=population;clock.dirty=0;clock.solves+=1;
    }
    needs.clothes=colony_needs::fulfillment(state,79);needs.beer=colony_needs::fulfillment(state,5);
    let fulfillment=food(state);let mut next=u64::MAX;
    for tier in tiers{tier.food=fulfillment;next=next.min(tier.next_with(needs.clothes));}
    if next==1{clock.next=clock.tick.saturating_add(1);return Ok(());}
    for r in 0..R{
        let rate=state[INPUT+N+r] as i64;
        let cap=if state[CAPS+r]==0{MAX}else{state[CAPS+r]-1};
        next=next.min(stock_boundary(state[stock_index(r)],rate,clock.carry[r],cap));
    }
    clock.next=clock.tick.saturating_add(next);
    Ok(())
}
pub fn advance(state:&mut [u64;WORDS],clock:&mut ClockState,tiers:&mut [PopulationTier],target:u64)->Result<(),()>{advance_with(state,clock,tiers,&ColonyNeeds::default(),target)}
pub fn advance_with(state:&mut [u64;WORDS],clock:&mut ClockState,tiers:&mut [PopulationTier],needs:&ColonyNeeds,target:u64)->Result<(),()>{
    if target<clock.tick{return Err(());}
    if clock.dirty!=0||state[READY]==0{return Err(());}
    let until=target.min(clock.next);let seconds=until.saturating_sub(clock.tick);
    if seconds==0{return Ok(());}
    for r in 0..R{
        let rate=state[INPUT+N+r] as i64;if rate==0{continue;}
        let index=stock_index(r);let before=state[index];let cap=if state[CAPS+r]==0{MAX}else{state[CAPS+r]-1};
        let after=project_stock(before,rate,&mut clock.carry[r],cap,seconds);
        if (before==0)!=(after==0)||(before>=cap)!=(after>=cap){clock.dirty=1;}
        state[index]=after;
    }
    for tier in tiers{tier.advance_with(seconds,needs.clothes);}
    clock.tick=until;
    // The caller refreshes after all state changes, so one instruction never solves twice.
    Ok(())
}
