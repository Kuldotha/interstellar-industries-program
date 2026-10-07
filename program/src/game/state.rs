use crate::{
    colony::{self, ClockState},
    population::PopulationTier,
    *,
};
use pinocchio::{error::ProgramError, ProgramResult};
pub const TOPOLOGY: &[u8] =
    include_bytes!("../../../planet-generation-bench/fixtures/fixed-topology-8.bin");
pub const TILES: usize = 642;
pub const RESOLUTION: u32 = 8;
pub const LEGACY_TAG:u64=u64::from_le_bytes(*b"IIWORLD1");
pub const PLANET_TAG: u64 = u64::from_le_bytes(*b"IIWORLD2");
pub const NEVER: u64 = u64::MAX;
pub const CONCRETE: usize = 16;
pub const HOUSE: u8 = 2;
pub const DOCK: u8 = 3;
pub const COMMONS: u8 = 4;
pub const RADIO: u8 = 11;
pub const CATCH_UP: u32 = 100;
pub fn invalid() -> ProgramError {
    ProgramError::InvalidAccountData
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Tile {
    pub kind: u8,
    pub facing: u8,
    pub paused: u8,
    pub paid: u8,
    pub coverage: u16,
    pub reserved: u16,
}
#[repr(C)]
pub struct Planet {
    pub tag: u64,
    pub owner: [u8; 32],
    pub sponsor: [u8; 32],
    pub nonce: u64,
    pub seed: u32,
    pub resolution: u32,
    pub permutation: [u8; 256],
    pub clock: ClockState,
    pub tier: PopulationTier,
    pub counts: [u64; 5],
    pub peak: u64,
    pub phase: u64,
    pub revision: u64,
    pub due: u64,
    pub task_id: u64,
    pub tiles: [Tile; TILES],
    pub session: [u8; 32],
    pub needs: colony_needs::ColonyNeeds,
}
pub fn recipe(kind:u8)->Option<usize>{colony_buildings::recipe(kind)}
pub fn workers(kind:u8)->u64{colony_buildings::workers(kind)}
pub fn cost(kind:u8)->u64{colony_buildings::cost(kind)}
pub fn neighbor(t: &[u8], id: usize, side: usize) -> usize {
    u32::from_le_bytes(
        t[20 + id * 36 + side * 4..24 + id * 36 + side * 4]
            .try_into()
            .unwrap(),
    ) as usize
}
pub fn region(t: &[u8], tile: usize) -> ([usize; 43], usize) {
    let mut ids = [usize::MAX; 43];
    ids[0] = tile;
    let mut count = 1;
    for side in 0..6 {
        let n = neighbor(t, tile, side);
        if n < TILES && !ids[..count].contains(&n) {
            ids[count] = n;
            count += 1;
        }
    }
    let first = count;
    for i in 1..first {
        for side in 0..6 {
            let n = neighbor(t, ids[i], side);
            if n < TILES && !ids[..count].contains(&n) {
                ids[count] = n;
                count += 1;
            }
        }
    }
    (ids, count)
}
impl Planet {
    pub fn initialize(
        &mut self,
        state: &mut [u64; WORDS],
        owner: [u8; 32],
        sponsor: [u8; 32],
        nonce: u64,
        seed: u32,
        now: u64,
    ) {
        self.tag = PLANET_TAG;
        self.owner = owner;
        self.sponsor = sponsor;
        self.nonce = nonce;
        self.seed = seed;
        self.resolution = RESOLUTION;
        planet_generation_core::shuffle(&mut self.permutation, seed);
        self.clock.tick = now;
        self.clock.next = NEVER;
        self.clock.dirty = 1;
        self.due = NEVER;
        state[stock_index(CONCRETE)] = 2 * Q;
        for r in 0..R {
            state[CAPS + r] = 100 * Q + 1;
        }
    }
    pub fn advance_for_action(&mut self, state: &mut [u64; WORDS], now: u64) -> ProgramResult {
        if now < self.clock.tick {
            return Err(invalid());
        }
        if state[READY] == 0 {
            self.clock.tick = now;
            return Ok(());
        }
        if now > self.clock.next {
            return Err(ProgramError::Custom(CATCH_UP));
        }
        colony::advance_with(
            state,
            &mut self.clock,
            core::slice::from_mut(&mut self.tier),
            &self.needs,
            now,
        )
        .map_err(|_| invalid())?;
        self.progress(state);
        Ok(())
    }
    pub fn refresh(&mut self, state: &mut [u64; WORDS]) -> ProgramResult {
        self.progress(state);
        colony::refresh_with(
            state,
            &mut self.clock,
            core::slice::from_mut(&mut self.tier),
            &mut self.needs,
        )
        .map_err(|_| invalid())?;
        self.progress(state);
        if self.clock.dirty!=0{colony::refresh_with(state,&mut self.clock,core::slice::from_mut(&mut self.tier),&mut self.needs).map_err(|_|invalid())?;}
        Ok(())
    }
    pub fn advance(&mut self, state: &mut [u64; WORDS], now: u64) -> ProgramResult {
        if state[READY] == 0 {
            self.clock.tick = now;
            return Ok(());
        }
        colony::advance_with(
            state,
            &mut self.clock,
            core::slice::from_mut(&mut self.tier),
            &self.needs,
            now,
        )
        .map_err(|_| invalid())?;
        self.refresh(state)
    }
    fn progress(&mut self, state: &mut [u64; WORDS]) {
        self.peak = self.peak.max(self.tier.population);
        for _ in 0..crate::colony_progression::COMPLETE {
            let full_radio=colony_needs::power_fulfillment(colony_needs::power(state),self.needs.power_demand)==Q;
            let ready=self.phase==10&&self.tier.food==Q&&self.needs.clothes==Q&&self.needs.beer==Q&&full_radio&&self.tier.population>=self.tier.capacity_with(self.needs.clothes)&&self.tiles.iter().any(|t|t.kind==HOUSE+1&&t.coverage>0&&t.reserved>0);
            let p=crate::colony_progression::Progress{houses:self.tier.houses,population:self.tier.population,fish:self.tier.food,quarries:self.counts[0],concrete:self.counts[1],commons:self.tier.commons_covered,fibers:state[8],tubers:state[10],clothes:self.needs.clothes,beer:self.needs.beer,biomass:state[12],generators:self.needs.generators,radio:if full_radio{self.needs.radio_covered}else{0},ready};
            if !crate::colony_progression::complete(self.phase,&p){break;}
            let reward=crate::colony_progression::reward(self.phase);
            state[stock_index(CONCRETE)] += reward * Q;
            self.phase += 1;
            // Quest grants can cross a stock boundary without elapsed production.
            if reward > 0 {
                self.clock.dirty = 1;
            }
        }
    }
    pub fn field_parent(&self,tile:usize)->Option<usize>{let item=self.tiles.get(tile)?;colony_buildings::farm_kind(item.kind.checked_sub(1)?)?;Some(neighbor(TOPOLOGY,tile,item.facing as usize))}
    pub fn fields(&self,tile:usize)->u64{(0..6).filter(|&side|{let n=neighbor(TOPOLOGY,tile,side);n<TILES&&self.field_parent(n)==Some(tile)}).count() as u64}
    fn production(&mut self, state: &mut [u64; WORDS], kind: u8, add: bool,tile:usize) {
        if let Some(r) = recipe(kind) {
            if add {
                state[r] += if colony_buildings::field_kind(kind).is_some(){self.fields(tile)*Q/3}else{Q};

            } else {
                state[r] -= if colony_buildings::field_kind(kind).is_some(){self.fields(tile)*Q/3}else{Q};

            }
        }
        if let Some(farm)=colony_buildings::farm_kind(kind){
            let parent=self.field_parent(tile).unwrap();let count=self.fields(parent);let r=recipe(farm).unwrap();
            if self.tiles[parent].paused==0{if add{state[r]+=count*Q/3-(count-1)*Q/3;}else{state[r]-=count*Q/3-(count-1)*Q/3;}}
        }
        if add {self.clock.workers+=workers(kind);self.needs.power_demand+=colony_buildings::power_demand(kind);if kind==10{self.needs.generators+=1;}}
        else {self.clock.workers-=workers(kind);self.needs.power_demand-=colony_buildings::power_demand(kind);if kind==10{self.needs.generators-=1;}}
        self.clock.dirty = 1;
    }
    fn utility(&mut self, t: &[u8], tile: usize, kind:u8, add: bool) {
        let (ids, count) = region(t, tile);
        for &id in &ids[..count] {
            let item = &mut self.tiles[id];
            let coverage=if kind==RADIO{&mut item.reserved}else{&mut item.coverage};
            let fulfilled=if kind==RADIO{&mut self.needs.radio_covered}else{&mut self.tier.commons_covered};
            let before = *coverage > 0;
            if add {
                *coverage += 1;
            } else {
                *coverage -= 1;
            }
            if item.kind == HOUSE + 1 {
                match (before, *coverage > 0) {
                    (false, true) => *fulfilled += 1,
                    (true, false) => *fulfilled -= 1,
                    _ => {}
                }
            }
        }
    }
    pub fn build(
        &mut self,
        state: &mut [u64; WORDS],
        t: &[u8],
        tile: usize,
        kind: u8,
        facing: usize,
    ) -> ProgramResult {
        if tile >= TILES || kind as usize >= colony_buildings::BUILDING_COUNT || facing >= 6 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if self.tiles[tile].kind != 0 {
            return Err(ProgramError::Custom(101));
        }
        let required = colony_buildings::unlock_population(kind);
        if self.peak < required {
            return Err(ProgramError::Custom(102));
        }
        let own = planet_generation_core::tile(t, tile, &self.permutation, self.seed);
        if own[1] == 0 || (kind == 0 && own[2] & 2 == 0) {
            return Err(ProgramError::Custom(103));
        }
        let adjacent = neighbor(t, tile, facing);
        if adjacent >= TILES {
            return Err(ProgramError::Custom(104));
        }
        if kind == DOCK
            && planet_generation_core::tile(t, adjacent, &self.permutation, self.seed)[1] != 0
        {
            return Err(ProgramError::Custom(104));
        }
        if let Some(farm)=colony_buildings::farm_kind(kind){if self.tiles[adjacent].kind!=farm+1||self.fields(adjacent)>=3{return Err(ProgramError::Custom(108));}}
        let price = cost(kind);
        let balance = &mut state[stock_index(CONCRETE)];
        if *balance < price * Q {
            return Err(ProgramError::Custom(105));
        }
        *balance -= price * Q;
        let cover = self.tiles[tile].coverage;
        let radio_cover=self.tiles[tile].reserved;
        self.tiles[tile] = Tile {
            kind: kind + 1,
            facing: facing as u8,
            paid: price as u8,
            coverage: cover,
            reserved: radio_cover,
            ..Tile::default()
        };
        if let Some(count)=self.counts.get_mut(kind as usize){*count+=1;}
        if kind == HOUSE {
            if radio_cover>0{self.needs.radio_covered+=1;}
            self.tier.houses += 1;
            self.tier.population += 2;
            if cover > 0 {
                self.tier.commons_covered += 1;
            }
        }
        if colony_buildings::utility(kind) {
            self.utility(t, tile, kind, true);
        }
        self.production(state, kind, true,tile);
        Ok(())
    }
    pub fn demolish(&mut self, state: &mut [u64; WORDS], t: &[u8], tile: usize) -> ProgramResult {
        let item = *self
            .tiles
            .get(tile)
            .ok_or(ProgramError::InvalidInstructionData)?;
        if item.kind == 0 {
            return Err(ProgramError::Custom(106));
        }
        let kind = item.kind - 1;
        if colony_buildings::field_kind(kind).is_some(){for side in 0..6{let n=neighbor(t,tile,side);if n<TILES&&self.field_parent(n)==Some(tile){self.demolish(state,t,n)?;}}}
        if colony_buildings::utility(kind) && item.paused == 0 {
            self.utility(t, tile, kind, false);
        }
        if kind == HOUSE {
            self.tier.population = self.tier.population * (self.tier.houses - 1) / self.tier.houses;
            self.tier.houses -= 1;
            if item.reserved>0{self.needs.radio_covered-=1;}
            if item.coverage > 0 {
                self.tier.commons_covered -= 1;
            }
            if self.tier.houses == 0 {
                self.tier = PopulationTier::default();
            }
        }
        if item.paused == 0 {
            self.production(state, kind, false,tile);
        }
        self.clock.dirty = 1;
        state[stock_index(CONCRETE)] += item.paid as u64 * Q;
        if let Some(count)=self.counts.get_mut(kind as usize){*count-=1;}
        self.tiles[tile] = Tile {
            coverage: self.tiles[tile].coverage,
            reserved: self.tiles[tile].reserved,
            ..Tile::default()
        };
        Ok(())
    }
    pub fn pause(
        &mut self,
        state: &mut [u64; WORDS],
        t: &[u8],
        tile: usize,
        paused: u8,
    ) -> ProgramResult {
        let item = *self
            .tiles
            .get(tile)
            .ok_or(ProgramError::InvalidInstructionData)?;
        if item.kind == 0 || item.kind == HOUSE + 1 || item.kind>=13 || paused > 1 {
            return Err(ProgramError::InvalidInstructionData);
        }
        if item.paused == paused {
            return Ok(());
        }
        let kind = item.kind - 1;
        if colony_buildings::utility(kind) {
            self.utility(t, tile, kind, paused == 0);
        }
        self.production(state, kind, paused == 0,tile);
        self.tiles[tile].paused = paused;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> (Box<Planet>, Box<[u64; WORDS]>) {
        let mut p: Box<Planet> = unsafe {
            Box::from_raw(std::alloc::alloc_zeroed(std::alloc::Layout::new::<Planet>()).cast())
        };
        let mut s = Box::new([0; WORDS]);
        p.initialize(&mut s, [1; 32], [2; 32], 1, 23, 100);
        (p, s)
    }
    fn site(p: &Planet, kind: u8) -> (usize, usize) {
        for id in 0..TILES {
            if p.tiles[id].kind != 0 {
                continue;
            }
            let t = planet_generation_core::tile(TOPOLOGY, id, &p.permutation, p.seed);
            if t[1] == 0 || (kind == 0 && t[2] & 2 == 0) {
                continue;
            }
            for side in 0..6 {
                let n = neighbor(TOPOLOGY, id, side);
                if n < TILES
                    && (kind != DOCK
                        || planet_generation_core::tile(TOPOLOGY, n, &p.permutation, p.seed)[1]
                            == 0)
                {
                    return (id, side);
                }
            }
        }
        panic!("no site")
    }
    #[test]
    fn fields_supply_only_their_farm_and_demolition_refunds_both(){
        let(mut p,mut s)=world();p.peak=100;s[stock_index(CONCRETE)]=100*Q;
        let farm=(0..TILES).find(|&id|planet_generation_core::tile(TOPOLOGY,id,&p.permutation,p.seed)[1]>0&&(0..6).filter(|&side|{let n=neighbor(TOPOLOGY,id,side);n<TILES&&planet_generation_core::tile(TOPOLOGY,n,&p.permutation,p.seed)[1]>0}).count()>=4).unwrap();
        p.build(&mut s,TOPOLOGY,farm,5,0).unwrap();assert_eq!(s[8],0);assert_eq!(p.clock.workers,5);
        let plots:Vec<_>=(0..6).map(|side|neighbor(TOPOLOGY,farm,side)).filter(|&n|n<TILES&&planet_generation_core::tile(TOPOLOGY,n,&p.permutation,p.seed)[1]>0).collect();
        for(i,&id)in plots.iter().take(3).enumerate(){let face=(0..6).find(|&j|neighbor(TOPOLOGY,id,j)==farm).unwrap();p.build(&mut s,TOPOLOGY,id,12,face).unwrap();assert_eq!(s[8],(i as u64+1)*Q/3);assert_eq!(p.clock.workers,5);}
        let id=plots[3];let face=(0..6).find(|&j|neighbor(TOPOLOGY,id,j)==farm).unwrap();assert!(p.build(&mut s,TOPOLOGY,id,12,face).is_err());
        p.pause(&mut s,TOPOLOGY,farm,1).unwrap();assert_eq!(s[8],0);p.demolish(&mut s,TOPOLOGY,plots[0]).unwrap();assert_eq!(s[8],0);
        p.pause(&mut s,TOPOLOGY,farm,0).unwrap();assert_eq!(s[8],2*Q/3);p.demolish(&mut s,TOPOLOGY,farm).unwrap();assert_eq!(s[8],0);assert_eq!(p.clock.workers,0);assert_eq!(p.fields(farm),0);assert_eq!(s[stock_index(CONCRETE)],100*Q);
    }
    #[test]
    fn layout_preserves_account_fields() {
        assert_eq!(core::mem::offset_of!(Planet,session),6288);
        assert_eq!(core::mem::offset_of!(Planet,needs),6320);
        assert_eq!(core::mem::size_of::<Planet>(),6360);
    }
    #[test]
    fn power_is_requested_by_radio_and_fuel_follows_demand() {
        let (mut p,mut s)=world();p.peak=100;s[stock_index(CONCRETE)]=100*Q;
        for _ in 0..5 {let(id,side)=site(&p,HOUSE);p.build(&mut s,TOPOLOGY,id,HOUSE,side).unwrap();}
        let(generator,side)=site(&p,10);p.build(&mut s,TOPOLOGY,generator,10,side).unwrap();
        let(radio,side)=site(&p,RADIO);p.build(&mut s,TOPOLOGY,radio,RADIO,side).unwrap();
        p.refresh(&mut s).unwrap();assert_eq!(colony_needs::power(&s),0);
        s[stock_index(6)]=Q;p.clock.dirty=1;p.refresh(&mut s).unwrap();
        assert_eq!(p.needs.power_demand,1);assert_eq!(colony_needs::power_fulfillment(colony_needs::power(&s),1),Q);
        p.pause(&mut s,TOPOLOGY,radio,1).unwrap();p.refresh(&mut s).unwrap();
        assert_eq!(p.needs.power_demand,0);assert_eq!(s[CONSUMED+6],0);
        p.pause(&mut s,TOPOLOGY,radio,0).unwrap();p.pause(&mut s,TOPOLOGY,generator,1).unwrap();p.refresh(&mut s).unwrap();
        assert_eq!(p.needs.generators,0);assert_eq!(p.clock.workers,0);assert_eq!(colony_needs::power(&s),0);
        p.demolish(&mut s,TOPOLOGY,radio).unwrap();assert_eq!(p.needs.power_demand,0);
    }
    #[test]
    fn empty_planet_has_no_timer_and_only_initial_concrete() {
        let (p, s) = world();
        assert_eq!(p.clock.next, NEVER);
        assert_eq!(p.due, NEVER);
        assert_eq!(s[stock_index(CONCRETE)], 2 * Q);
        assert_eq!(s[READY], 0);
        assert_eq!(planet_generation_core::tiles(TOPOLOGY), TILES);
        assert!(core::mem::size_of::<Planet>() <= 10240);
        assert!(WORDS * 8 <= 10240);
        println!(
            "planet={} engine={} sponsor=56 topology={}",
            core::mem::size_of::<Planet>(),
            WORDS * 8,
            32 + TOPOLOGY.len()
        );
    }
    #[test]
    fn bootstrap_then_growth_stocks_and_stale_action() {
        let (mut p, mut s) = world();
        for _ in 0..2 {
            let (id, side) = site(&p, HOUSE);
            p.advance_for_action(&mut s, 500).unwrap();
            p.build(&mut s, TOPOLOGY, id, HOUSE, side).unwrap();
            p.refresh(&mut s).unwrap();
        }
        assert_eq!(p.clock.next, NEVER);
        assert_eq!(p.tier.population, 4);
        assert_eq!(s[stock_index(CONCRETE)], Q);
        let (id, side) = site(&p, DOCK);
        p.build(&mut s, TOPOLOGY, id, DOCK, side).unwrap();
        p.refresh(&mut s).unwrap();
        assert_eq!(p.clock.next, 506);
        assert_eq!(
            p.advance_for_action(&mut s, 510),
            Err(ProgramError::Custom(CATCH_UP))
        );
        for _ in 0..3 {
            p.advance(&mut s, 600).unwrap();
        }
        assert_eq!(p.tier.population, 10);
        assert_eq!(p.phase, 2);
        assert_eq!(s[stock_index(CONCRETE)], 2 * Q);
        let before = p.clock.tick;
        p.advance(&mut s, before).unwrap();
        assert_eq!(p.phase, 2);
        p.pause(&mut s, TOPOLOGY, id, 1).unwrap();
        assert_eq!(p.clock.workers, 0);
        p.demolish(&mut s, TOPOLOGY, id).unwrap();
        assert_eq!(s[stock_index(CONCRETE)], 3 * Q);
    }
    #[test]
    fn coverage_counts_union_and_releases_only_last_provider() {
        let (mut p, mut s) = world();
        p.peak = 20;
        s[stock_index(CONCRETE)] = 100 * Q;
        let candidates: Vec<_> = (0..TILES)
            .filter(|&i| planet_generation_core::tile(TOPOLOGY, i, &p.permutation, p.seed)[1] != 0)
            .collect();
        let h = candidates
            .iter()
            .copied()
            .find(|&id| {
                let (r, n) = region(TOPOLOGY, id);
                r[..n].iter().filter(|i| candidates.contains(i)).count() >= 3
            })
            .unwrap();
        let (r, n) = region(TOPOLOGY, h);
        let others: Vec<_> = r[..n]
            .iter()
            .copied()
            .filter(|&i| i != h && candidates.contains(&i))
            .take(2)
            .collect();
        let side = |id| (0..6).find(|&j| neighbor(TOPOLOGY, id, j) < TILES).unwrap();
        p.build(&mut s, TOPOLOGY, h, HOUSE, side(h)).unwrap();
        for id in &others {
            p.build(&mut s, TOPOLOGY, *id, COMMONS, side(*id)).unwrap();
        }
        assert_eq!(p.tier.commons_covered, 1);
        p.demolish(&mut s, TOPOLOGY, others[0]).unwrap();
        assert_eq!(p.tier.commons_covered, 1);
        p.pause(&mut s, TOPOLOGY, others[1], 1).unwrap();
        assert_eq!(p.tier.commons_covered, 0);
    }
}
