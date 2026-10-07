use crate::Q;
pub const COMPLETE:u64=11;
#[derive(Default)]
pub struct Progress {pub houses:u64,pub population:u64,pub fish:u64,pub quarries:u64,pub concrete:u64,pub commons:u64,pub fibers:u64,pub tubers:u64,pub clothes:u64,pub beer:u64,pub biomass:u64,pub generators:u64,pub radio:u64,pub ready:bool}
pub fn complete(stage:u64,p:&Progress)->bool{match stage{
0=>p.houses>=2,1=>p.population>=10&&p.fish==Q,2=>p.quarries>0&&p.concrete>0,3=>p.population>=20,4=>p.commons>=4,
5=>p.population>=60,6=>p.fibers>=Q&&p.tubers>=Q,7=>p.clothes==Q&&p.beer==Q,8=>p.population>=100,
9=>p.biomass>=Q&&p.generators>0&&p.radio>0,10=>p.ready,_=>false}}
pub fn reward(stage:u64)->u64{match stage{0=>1,1|2=>2,3=>3,5=>12,6=>10,8=>20,_=>0}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn readiness_requires_every_need_and_full_habitation(){
  let mut p=Progress{houses:10,population:150,fish:Q,quarries:1,concrete:1,commons:10,fibers:Q,tubers:Q,clothes:Q,beer:Q,biomass:Q,generators:1,radio:10,ready:false};
  assert!(!complete(10,&p));p.ready=true;assert!(complete(10,&p));assert!(!complete(COMPLETE,&p));
  let mut phase=0;let mut grants=0;while complete(phase,&p){grants+=reward(phase);phase+=1;}assert_eq!(phase,COMPLETE);assert_eq!(grants,50);
  p.clothes=Q-1;assert!(!complete(7,&p));p.clothes=Q;p.beer=0;assert!(!complete(7,&p));
 }
}
