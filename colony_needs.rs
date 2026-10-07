use crate::{Q,R,population::workforce_capacity};
#[repr(C)]
#[derive(Clone,Copy,Default,Debug,PartialEq,Eq)]
pub struct ColonyNeeds {pub clothes:u64,pub beer:u64,pub radio_covered:u64,pub generators:u64,pub power_demand:u64}
pub fn demands(population:u64,workers:u64,generators:u64,power_demand:u64)->[u64;R]{
 let mut demand=[0;R];demand[23]=population*Q/50;demand[79]=population*Q/100;demand[5]=population*Q/100;
 demand[6]=workforce_capacity(generators*Q,population,workers).min((power_demand*Q+9)/10);demand
}
pub fn fulfillment(state:&[u64;crate::WORDS],resource:usize)->u64{let need=state[crate::CONSUMPTION+resource];if need==0{0}else{crate::mul_div(state[crate::CONSUMED+resource],Q,need).min(Q)}}
pub fn power(state:&[u64;crate::WORDS])->u64{state[crate::CONSUMED+6]*10}

pub fn power_fulfillment(supply:u64,demand:u64)->u64 {if demand==0{Q}else{crate::mul_div(supply,Q,demand*Q).min(Q)}}
