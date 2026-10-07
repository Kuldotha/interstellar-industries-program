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

#[cfg(test)]mod module_tests{use super::*;
#[test]fn modules_multiply_capacity_and_fuel_with_shared_workforce(){let c=[0;N];let a=plan(c,10,10,1,100,10*Q);let b=plan(c,25,25,4,100,10*Q);assert_eq!(a.supply,10*Q);assert_eq!(b.supply,40*Q);assert_eq!(b.fuel,4*a.fuel);let short=plan(c,10,25,4,100,10*Q);assert!(16*Q-short.supply<=10);let idle=plan(c,10,10,4,0,10*Q);assert_eq!(idle.fuel,0);}
}
