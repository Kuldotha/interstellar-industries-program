#![allow(dead_code)]
include!("../../shared.rs");
static mut STATE:[u64;WORDS]=[0;WORDS];
#[no_mangle]
pub extern "C" fn state_ptr()->*mut u64 {core::ptr::addr_of_mut!(STATE).cast()}
#[no_mangle]
pub extern "C" fn consumption_offset()->usize{CONSUMPTION}
#[no_mangle]
pub extern "C" fn consumed_offset()->usize{CONSUMED}
#[no_mangle]
pub extern "C" fn caps_offset()->usize{CAPS}
#[no_mangle]
pub extern "C" fn state_words()->usize{WORDS}
#[no_mangle]
pub extern "C" fn build(action:usize)->u32 {
    let state=unsafe{&mut *(state_ptr() as *mut [u64;WORDS])};
    if prepare_stock(state).is_err(){return 3;}
    let Ok(change)=prepare(state,action) else{return 1};
    if execute(state,change).is_err(){return 2;}
    0
}
#[no_mangle]
pub extern "C" fn recalculate()->u32 {build(255)}
#[no_mangle]
pub extern "C" fn reference()->u32 {
    let state=unsafe{&mut *(state_ptr() as *mut [u64;WORDS])};
    if state[CAPS..].iter().any(|&v|v!=0){return build(255);}
    let mut cap=[0;N];for i in 0..N{cap[i]=effective(state[i],state[FACTOR]);}
    let stock=unsafe{&*(state.as_ptr().add(N) as *const [u64;RAW])};
    let output=unsafe{&mut *(state.as_mut_ptr().add(INPUT) as *mut Solution)};
    if has_intermediates(state){let inventory=stocks(state);let meta=unsafe{&*(state.as_ptr().add(METADATA) as *const active::StockMetadata)};if active::solve_stock(&[true;N],&[true;R],&cap,&inventory,output,meta).is_err(){return 2;}let mut next=u64::MAX;for r in 0..R{if output.balance[r]<0{let draw=(-output.balance[r]) as u64;if inventory[r]==0{return 2;}next=next.min(mul_div(inventory[r],Q,draw));}}output.next=next;}else if solve_selected((1<<COMPONENTS)-1,&cap,stock,output).is_err(){return 2;}0
}
static mut CLOCK:colony::ClockState=colony::ClockState{tick:0,next:0,workers:0,population:0,solves:0,dirty:1,carry:[0;R]};
static mut TIERS:[population::PopulationTier;8]=[population::PopulationTier{houses:0,population:0,commons_covered:0,food:0,growth:0};8];
#[no_mangle] pub extern "C" fn colony_clock_ptr()->*mut u64{core::ptr::addr_of_mut!(CLOCK).cast()}
#[no_mangle] pub extern "C" fn colony_tiers_ptr()->*mut u64{core::ptr::addr_of_mut!(TIERS).cast()}
#[no_mangle] pub extern "C" fn colony_clock_words()->usize{colony::CLOCK_WORDS}
#[no_mangle] pub extern "C" fn colony_update(count:usize,target:u64,initialize:u32)->u32{
 if count>8||initialize>1{return 1;}
 let state=unsafe{&mut *core::ptr::addr_of_mut!(STATE)};
 let clock=unsafe{&mut *core::ptr::addr_of_mut!(CLOCK)};
 let tiers=unsafe{core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(TIERS).cast(),count)};
 if initialize==1&&state[READY]==0&&clock.solves==0{clock.tick=target;}
 if initialize==0&&colony::advance(state,clock,tiers,target).is_err(){return 2;}
 if colony::refresh(state,clock,tiers).is_err(){return 3;}0
}
