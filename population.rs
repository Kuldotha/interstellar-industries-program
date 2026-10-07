#[repr(C)]
#[derive(Clone,Copy,Default,Debug,PartialEq,Eq)]
pub struct PopulationTier {
    pub houses:u64,
    pub population:u64,
    pub commons_covered:u64,
    pub food:u64,
    pub growth:u64,
}
impl PopulationTier {
    pub fn valid(&self)->bool{self.houses<=32768&&self.population<=self.houses*15&&self.commons_covered<=self.houses&&self.food<=crate::Q&&(self.growth==0||(100..106).contains(&self.growth)||(200..202).contains(&self.growth))}
    pub fn capacity(&self)->u64{2*self.houses+5*self.commons_covered+3*self.houses*self.food/crate::Q}
    pub fn utility_fulfillment(&self)->u64{if self.houses==0{0}else{self.commons_covered*crate::Q/self.houses}}
    pub fn capacity_with(&self,clothes:u64)->u64{self.capacity()+5*self.houses*clothes/crate::Q}
    fn direction(&self,clothes:u64)->u64{let target=self.capacity_with(clothes);u64::from(self.population<target)+2*u64::from(self.population>target)}
    pub fn next(&self)->u64{self.next_with(0)}
    pub fn next_with(&self,clothes:u64)->u64{
        let direction=self.direction(clothes);if direction==0{return u64::MAX;}
        let elapsed=if self.growth/100==direction{self.growth%100}else{0};
        (if direction==1{6u64}else{2u64}).saturating_sub(elapsed).max(1)
    }
    pub fn advance(&mut self,seconds:u64){self.advance_with(seconds,0)}
    pub fn advance_with(&mut self,seconds:u64,clothes:u64){
        let direction=self.direction(clothes);if direction==0{self.growth=0;return;}
        let target=self.capacity_with(clothes);let period=if direction==1{6}else{2};
        let elapsed=if self.growth/100==direction{self.growth%100}else{0};
        let total=elapsed.saturating_add(seconds);let change=(total/period).saturating_mul(self.houses);
        if direction==1{self.population=self.population.saturating_add(change).min(target);}else{self.population=self.population.saturating_sub(change).max(target);}
        self.growth=if self.population==target{0}else{direction*100+total%period};
    }
}
pub fn workforce_capacity(total_capacity:u64,population:u64,workers:u64)->u64{
    if workers==0{total_capacity}else{crate::mul_div(total_capacity,population.min(workers),workers)}
}
pub fn fish_demand(population:u64)->u64{population*crate::Q/50}
#[inline(always)]
pub fn project_stock(stock:u64,rate:i64,carry:&mut i64,cap:u64,seconds:u64)->u64{
    let delta=if seconds>i64::MAX as u64{None}else if seconds==1{Some(rate+*carry)}else{rate.checked_mul(seconds as i64).and_then(|v|v.checked_add(*carry))};
    if let Some(delta)=delta{
        let magnitude=delta.unsigned_abs();let whole=magnitude/core::hint::black_box(60u64);let remainder=(magnitude-whole*60) as i64;
        if delta<0{*carry=-remainder;return stock.saturating_sub(whole);}
        *carry=remainder;return stock.saturating_add(whole).min(cap.max(stock));
    }
    let delta=rate as i128*seconds as i128+*carry as i128;
    *carry=(delta%60) as i64;
    (stock as i128+delta/60).max(0).min(cap.max(stock) as i128) as u64
}
pub fn stock_boundary(stock:u64,rate:i64,carry:i64,cap:u64)->u64{
    if rate==0{return u64::MAX;}
    let numerator=if rate<0{stock as i64*60+carry}else{cap.saturating_sub(stock) as i64*60-carry};
    let divisor=rate.unsigned_abs();
    ((numerator.max(0) as u64+divisor-1)/divisor).max(1)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn bonuses_accumulate_before_rounding(){let t=PopulationTier{houses:10,population:20,commons_covered:4,food:crate::Q/2,growth:0};assert_eq!(t.capacity(),55);assert_eq!(t.utility_fulfillment(),crate::Q*4/10);}
    #[test] fn grows_and_declines_as_one_tier(){let mut t=PopulationTier{houses:10,population:20,commons_covered:4,food:crate::Q/2,growth:0};assert_eq!(t.next(),6);t.advance(5);assert_eq!(t.population,20);assert_eq!(t.next(),1);t.advance(1);assert_eq!(t.population,30);t.advance(18);assert_eq!(t.population,55);assert_eq!(t.next(),u64::MAX);t.food=0;t.commons_covered=0;t.advance(8);assert_eq!(t.population,20);}
    #[test] fn fixed_supply_batch_equals_individual_seconds(){for houses in [1,7,100,2000]{let mut a=PopulationTier{houses,population:2*houses,commons_covered:houses/3,food:crate::Q/2,growth:0};let mut b=a;a.advance(86400);for _ in 0..86400{b.advance(1);}assert_eq!(a,b);a.food=0;b.food=0;a.advance(60);for _ in 0..60{b.advance(1);}assert_eq!(a,b);}}
    #[test] fn direction_change_does_not_reuse_growth_credit(){let mut t=PopulationTier{houses:10,population:30,commons_covered:0,food:crate::Q,growth:105};t.food=0;t.advance(1);assert_eq!(t.population,30);assert_eq!(t.growth,201);t.advance(1);assert_eq!(t.population,20);}
    #[test] fn empty_and_invalid_tiers(){assert_eq!(PopulationTier::default().capacity(),0);assert!(PopulationTier::default().valid());assert!(!PopulationTier{houses:1,commons_covered:2,..Default::default()}.valid());assert!(!PopulationTier{houses:1,food:crate::Q+1,..Default::default()}.valid());}
}
