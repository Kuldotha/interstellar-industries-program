struct StockRow {full:u64,a:&'static [(usize,i64,u64)],b:&'static [(usize,i64)]}
struct StockRoute {variable:usize,recipe:usize,positive:&'static [StockRow],negative:&'static [StockRow],demand:&'static [(usize,i64,u64)],producers:&'static [usize]}
struct StockColumn {row:usize,terms:&'static [(i64,u64)]}
struct StockPlan {columns:&'static [&'static [StockColumn]],mask:u64,rows:&'static [StockRow],routes:&'static [StockRoute]}
include!("stock_data.rs");
const STOCK_LINKS:usize=640;
#[repr(C)]
pub struct StockMetadata {ready:u64,stocked:u64,full:u64,masks:[u64;STOCK_ROWS],offsets:[u16;O+1],links:[u32;STOCK_LINKS]}
impl StockMetadata {
 pub const ZERO:Self=Self{ready:0,stocked:0,full:0,masks:[0;STOCK_ROWS],offsets:[0;O+1],links:[0;STOCK_LINKS]};
 fn group_index(&self)->usize {STOCK_PLANS.iter().position(|p|p.mask==self.stocked&STOCK_VARIANT_BITS).unwrap()}
 fn group_rows(&self)->&'static [usize] {STOCK_ROW_GROUPS[self.group_index()]}
 fn group_count(&self)->usize {STOCK_GROUP_COUNTS[self.group_index()]}
 fn plan(&self)->&'static StockPlan {STOCK_PLANS.iter().find(|p|p.mask==self.stocked&STOCK_VARIANT_BITS).unwrap()}
 pub fn prepare(&mut self,stocks:&[u64;R],full:u64)->Result<(),()> {
  let mut mask=0;for v in 0..O{if stocks[RECIPE_RESOURCE[OUTPUT_PRODUCERS[v][0]]]>0{mask|=1<<v;}}
  if self.ready==2&&self.stocked==mask&&self.full==full{return Ok(());}
  self.ready=0;self.stocked=mask;self.full=full;self.masks.fill(0);let columns=STOCK_GROUP_COLUMNS[self.group_index()];let mut used=0;
  for v in 0..O{self.offsets[v]=used as u16;for &(j,terms) in columns[v]{let c:i64=terms.iter().filter(|&&(_,m)|m&mask==0).map(|&(c,_)|c).sum();if c< -255||c>255{return Err(());}if c==0{continue;}if used==STOCK_LINKS||j>255{return Err(());}self.links[used]=((c as i32 as u32)<<8)|j as u32;used+=1;if c>0{self.masks[j]|=1<<v;}}}
  self.offsets[O]=used as u16;self.ready=2;Ok(())
 }
}
fn stock_dot(t:&[(usize,i64,u64)],v:&[u64],mask:u64)->i64{t.iter().filter(|&&(_,_,m)|m&mask==0).map(|&(i,c,_)|c*v[i] as i64).sum()}
fn stock_bound(row:&StockRow,cap:&[u64;N],stocked:&[bool;N],full:u64)->Option<u64>{if row.full&full!=row.full{return None;}if row.b.iter().any(|&(i,_)|stocked[i]){None}else{Some(dot(row.b,cap) as u64)}}
fn column_len<const STOCK:bool>(v:usize,meta:&StockMetadata)->usize{if STOCK{(meta.offsets[v+1]-meta.offsets[v]) as usize}else{GROUP_COLUMNS[v].len()}}
fn column<const STOCK:bool>(v:usize,k:usize,meta:&StockMetadata)->(usize,i64){if STOCK{let link=meta.links[meta.offsets[v] as usize+k];((link&255) as usize,((link as i32)>>8) as i64)}else{GROUP_COLUMNS[v][k]}}
#[inline(never)]
pub fn solve_stock(selected:&[bool;N],touched:&[bool;R],cap:&[u64;N],stocks:&[u64;R],out:&mut Solution,meta:&StockMetadata)->Result<(),()> {
 let stamp=crate::profile::start();
 if cap.iter().chain(stocks).any(|&v|v>MAX){return Err(());}
 let mut stocked=[false;N];for j in 0..RAW{stocked[RAW_RECIPES[j]]=stocks[RAW_RESOURCES[j]]>0;}
 let mut values=[0u64;52];let mut enabled=0u64;for v in 0..O{if OUTPUT_PRODUCERS[v].iter().any(|&i|cap[i]>0){enabled|=1<<v;}}
 crate::profile::end(3,stamp);let stamp=crate::profile::start();
 out.stages=allocate_values::<true>(enabled,cap,&stocked,&mut values,meta)?;
 crate::profile::end(4,stamp);let stamp=crate::profile::start();
 for route in meta.plan().routes {
  let mut low=0i64;let mut high=i64::MAX;
  for (positive,rows) in [(true,route.positive),(false,route.negative)]{for row in rows{if let Some(b)=stock_bound(row,cap,&stocked,meta.full){let coefficient=row.a.iter().find(|&&(i,_,_)|i==route.variable).ok_or(())?.1;let numerator=b as i64-stock_dot(row.a,&values,meta.stocked);let(n,d)=if coefficient>0{(numerator,coefficient)}else{(-numerator,-coefficient)};let floor=n.div_euclid(d);if positive{high=high.min(floor);}else{low=low.max(floor+i64::from(n.rem_euclid(d)>0));}}}}
  if low>high{return Err(());}let total:u64=route.producers.iter().map(|&i|cap[i]).sum();let demand=stock_dot(route.demand,&values,meta.stocked);let preferred=if total>0{mul_div(demand as u64,cap[route.recipe],total) as i64}else{0};values[route.variable]=preferred.clamp(low,high) as u64;
 }
 crate::profile::end(5,stamp);let stamp=crate::profile::start();
 for i in 0..N{if !selected[i]{continue;}let gross=if RAW_RECIPES.contains(&i){cap[i] as i64}else{stock_dot(STOCK_EXPRESSIONS[i],&values,meta.stocked)};if gross<0||gross>cap[i] as i64{return Err(());}out.gross[i]=gross as u64;}
 crate::profile::end(6,stamp);let stamp=crate::profile::start();
 for r in 0..R{if touched[r]{out.balance[r]=RESOURCE_PRODUCERS[r].iter().map(|&i|out.gross[i] as i64).sum::<i64>()-RESOURCE_CONSUMERS[r].iter().map(|&i|out.gross[i] as i64).sum::<i64>();}}
 crate::profile::end(7,stamp);
 Ok(())
}
