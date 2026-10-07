use crate::{Q,N,mul_div,population::workforce_capacity,colony_buildings::POWERED_RECIPES};
pub struct PowerPlan{pub capacities:[u64;N],pub fuel:u64,pub supply:u64,pub required_fraction:u64,pub factory_fraction:u64,pub demand:u64}
pub fn plan(mut capacities:[u64;N],population:u64,workers:u64,generators:u64,required:u64,stock:u64)->PowerPlan{
 let optional=POWERED_RECIPES.iter().map(|&r|capacities[r]).sum::<u64>();
 let demand=required*Q+optional;
 let mut fuel=workforce_capacity(generators*Q,population,workers).min((demand+9)/10);
 if stock==0{fuel=fuel.min(workforce_capacity(capacities[12],population,workers));}
 let supply=(fuel*10).min(demand);
 let required_fraction=if required==0{Q}else{mul_div(supply,Q,required*Q).min(Q)};
 let factory_fraction=if optional==0{Q}else{mul_div(supply.saturating_sub(required*Q),Q,optional).min(Q)};
 for r in POWERED_RECIPES{capacities[r]=mul_div(capacities[r],Q/2+factory_fraction/2,Q);}
 PowerPlan{capacities,fuel,supply,required_fraction,factory_fraction,demand}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn required_consumers_take_priority_and_factories_keep_half_output(){let mut c=[0;N];c[1]=2*Q;c[12]=Q;let off=plan(c,100,10,0,1,0);assert_eq!(off.capacities[1],Q);let full=plan(c,100,10,1,1,0);assert_eq!(full.capacities[1],2*Q);let short=plan(c,1,10,1,1,0);assert!(Q-short.required_fraction<=10);assert_eq!(short.factory_fraction,0);assert_eq!(short.capacities[1],Q);}
}
