#![allow(dead_code,unexpected_cfgs)]
include!("../../shared.rs");
#[derive(Clone)]
pub struct Engine {state:Box<[u64;WORDS]>}
impl Default for Engine {fn default()->Self{Self{state:Box::new([0;WORDS])}}}
impl Engine {
 pub fn calculate(&mut self,capacity:[u64;N],inventory:[u64;R],limits:[u64;R],available:u64,demand:[u64;R])->Result<(),()> {
  self.state[..N].copy_from_slice(&capacity);self.state[AVAILABLE]=available;
  for r in 0..R{self.state[stock_index(r)]=inventory[r];self.state[CAPS+r]=limits[r];self.state[CONSUMPTION+r]=demand[r];}
  prepare_stock(&mut self.state)?;let change=prepare(&mut self.state,255)?;execute(&mut self.state,change)
 }
 pub fn rate(&self,recipe:usize)->u64{self.state[INPUT+recipe]}
 pub fn balance(&self,resource:usize)->i64{self.state[INPUT+N+resource] as i64}
 pub fn fulfillment(&self,resource:usize)->u64{let need=self.state[CONSUMPTION+resource];if need==0{Q}else{mul_div(self.state[CONSUMED+resource],Q,need)}}
 pub fn food_fulfillment(&self)->u64{colony::food(&self.state)}
 pub fn next(&self)->u64{self.state[INPUT+N+R]}
}

#[cfg(test)]
mod needs_tests {
 use super::*;
 fn calculate(resource:usize,stock:u64,cap:u64,producer:u64)->Engine {
  let mut e=Engine::default();let mut capacity=[0;N];capacity[8]=producer;capacity[9]=producer;
  let mut inventory=[0;R];inventory[resource]=stock;
  let mut limits=[0;R];limits[resource]=cap+1;
  let mut demand=[0;R];demand[resource]=Q/2;
  e.calculate(capacity,inventory,limits,MAX,demand).unwrap();e
 }
 #[test] fn clothes_consumption_uses_output_then_stock() {
  let e=calculate(79,Q,10*Q,Q);assert_eq!(e.fulfillment(79),Q);assert_eq!(e.balance(79),Q as i64/2);
  let e=calculate(79,Q,10*Q,0);assert_eq!(e.fulfillment(79),Q);assert_eq!(e.balance(79),-(Q as i64)/2);assert_eq!(e.next(),2*Q);
  let e=calculate(79,0,10*Q,Q/4);assert_eq!(e.fulfillment(79),Q/2);assert_eq!(e.balance(79),0);
 }
 #[test] fn full_clothes_storage_replenishes_consumption() {
  let e=calculate(79,10*Q,10*Q,Q);assert_eq!(e.rate(9),Q/2);assert_eq!(e.balance(79),0);assert_eq!(e.fulfillment(79),Q);
 }
 #[test] fn power_bootstraps_without_power_and_tracks_fuel() {
  assert_eq!(colony_buildings::power_demand(9),0);assert_eq!(colony_buildings::power_demand(10),0);assert_eq!(colony_buildings::power_demand(11),1);
  for kind in 0..9 {assert_eq!(colony_buildings::power_demand(kind),0);}
  let mut e=Engine::default();let mut capacity=[0;N];capacity[12]=Q;
  let demand=colony_needs::demands(100,15,1,1);
  e.calculate(capacity,[0;R],[0;R],MAX,demand).unwrap();
  assert_eq!(colony_needs::power_fulfillment(colony_needs::power(&e.state),1),Q);
  e.calculate([0;N],[0;R],[0;R],MAX,demand).unwrap();assert_eq!(colony_needs::power(&e.state),0);
  assert_eq!(colony_needs::power_fulfillment(Q/2,1),Q/2);
  assert_eq!(colony_needs::demands(100,15,1,0)[6],0);
 }
}
