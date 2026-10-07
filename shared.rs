mod profile;
include!("incremental_kernel.rs");
mod active {include!("active_kernel.rs");}
pub const INPUT:usize=N+RAW;
pub const AVAILABLE:usize=282;
pub const FACTOR:usize=283;
pub const READY:usize=284;
pub const MODE:usize=285;
pub const DIRTY:usize=286;
pub const GROUP_ACTION:usize=287;
pub const STOCK:usize=371;
pub const DRAW:usize=451;
pub const METADATA:usize=531;
pub const CAPS:usize=METADATA+core::mem::size_of::<active::StockMetadata>()/8;
pub const CONSUMPTION:usize=CAPS+R;
pub const CONSUMED:usize=CONSUMPTION+R;
pub const WORDS:usize=CONSUMED+R;
fn cap_full(state:&[u64;WORDS],r:usize)->bool {state[CAPS+r]>0&&state[stock_index(r)]>=state[CAPS+r]-1}
fn full_outputs(state:&[u64;WORDS])->u64 {let mut mask=0;for v in 0..O{if cap_full(state,RECIPE_RESOURCE[OUTPUT_PRODUCERS[v][0]]) && state[CONSUMPTION+RECIPE_RESOURCE[OUTPUT_PRODUCERS[v][0]]]==0{mask|=1<<v;}}mask}
fn use_stock_solver(state:&[u64;WORDS])->bool {has_intermediates(state)||full_outputs(state)!=0}
const STOCK_INDICES:[usize;R]={let mut indices=[0;R];let mut r=0;while r<R{indices[r]=STOCK+r;r+=1;}let mut j=0;while j<RAW{indices[RAW_RESOURCES[j]]=N+j;j+=1;}indices};
#[inline(always)]
pub fn stock_index(r:usize)->usize {STOCK_INDICES[r]}
fn stocks(state:&[u64;WORDS])->[u64;R]{let mut result=[0;R];for r in 0..R{result[r]=state[STOCK+r];}for j in 0..RAW{result[RAW_RESOURCES[j]]=state[N+j];}result}
fn has_intermediates(state:&[u64;WORDS])->bool {state[STOCK..STOCK+R].iter().any(|&v|v>0)}
pub fn prepare_stock(state:&mut [u64;WORDS])->Result<(),()>{
 if state[STOCK..STOCK+R].iter().any(|&v|v>MAX){return Err(());}
 if state[CAPS..CAPS+R].iter().any(|&v|v>MAX+1){return Err(());}
 if (0..R).any(|r|state[CONSUMPTION+r]>MAX||(state[CONSUMPTION+r]>0&&!RAW_RESOURCES.contains(&r)&&!RESOURCE_CONSUMERS[r].is_empty())){return Err(());}
 if !use_stock_solver(state){return Ok(());}
 let inventory=stocks(state);let full=full_outputs(state);let meta=unsafe{&mut *(state.as_mut_ptr().add(METADATA) as *mut active::StockMetadata)};meta.prepare(&inventory,full)
}
fn effective(value:u64,factor:u64)->u64 {if factor==Q{value}else{(value>>24)*factor+(((value&(Q-1))*factor)>>24)}}
pub struct Change {pub previous:u64,pub mask:u64,pub direct:usize,pub full:bool,pub built:usize}
pub fn prepare(state:&mut [u64;WORDS],action:usize)->Result<Change,()> {
    if state[..INPUT].iter().any(|&v|v>MAX)||state[AVAILABLE]>MAX{return Err(());}
    if action!=254 && action!=255 && (action>=N || state[action]>MAX-Q){return Err(());}
    let previous=state[FACTOR];
    if action<N{state[action]+=Q;}
    let demand:u64=state[..N].iter().sum();
    let factor=if demand==0||state[AVAILABLE]>=demand{Q}else{mul_div(state[AVAILABLE],Q,demand)};
    state[FACTOR]=factor;
    let full=state[READY]==0||action==255||state[CONSUMPTION..CONSUMPTION+R].iter().any(|&v|v>0);
    let mut mask:u64=if full{(1<<COMPONENTS)-1}else{0};
    if !full && previous!=factor {for i in 0..N {
        let old=state[i]-if i==action{Q}else{0};
        if effective(old,previous)!=effective(state[i],factor){mask|=1<<RECIPE_COMPONENT[i];}
    }
    }
    if action<N{mask|=1<<RECIPE_COMPONENT[action];}
    let direct=if !full && previous==factor && action<N && RAW_RECIPES.contains(&action) {
        let resource=RECIPE_RESOURCE[action];
        let demand:u64=RESOURCE_CONSUMERS[resource].iter().map(|&i|effective(state[i],factor)).sum();
        let stock=RAW_RECIPES.iter().position(|&i|i==action).map(|j|state[N+j]).unwrap_or(0);
        if effective(state[action]-Q,factor)>=demand || stock>0{action}else{N}
    }else{N};
    state[DIRTY]=mask.count_ones() as u64;
    Ok(Change{previous,mask,direct,full,built:action})
}
#[inline(never)]
pub fn execute(state:&mut [u64;WORDS],change:Change)->Result<(),()> {
    let dispatch_stamp=profile::start();
    state[GROUP_ACTION..GROUP_ACTION+N].fill(0);
    let mut cap=[0;N];
    for i in 0..N{cap[i]=effective(state[i],state[FACTOR]);}
    for r in 0..R{if state[CONSUMPTION+r]>0&&RESOURCE_CONSUMERS[r].is_empty()&&cap_full(state,r){let total:u64=RESOURCE_PRODUCERS[r].iter().map(|&i|cap[i]).sum();let limit=state[CONSUMPTION+r];if total>limit{for &i in RESOURCE_PRODUCERS[r]{cap[i]=mul_div(cap[i],limit,total);}}}}
    state[CONSUMED..CONSUMED+R].fill(0);
    let mut reserved=[0u64;RAW];
    for j in 0..RAW{let r=RAW_RESOURCES[j];let need=state[CONSUMPTION+r];reserved[j]=need.min(cap[RAW_RECIPES[j]]);cap[RAW_RECIPES[j]]-=reserved[j];state[CONSUMED+r]=if state[stock_index(r)]>0{need}else{reserved[j]};}
    let p=state.as_mut_ptr();
    let stock=unsafe{&*(p.add(N) as *const [u64;RAW])};
    let output=unsafe{&mut *(p.add(INPUT) as *mut Solution)};
    let mut mask=change.mask;
    let mut scaled=0;
    if change.direct<N {
        let i=change.direct;let r=RECIPE_RESOURCE[i];
        output.balance[r]+=cap[i] as i64-output.gross[i] as i64;output.gross[i]=cap[i];
        state[GROUP_ACTION+i]=2;
        mask=0;state[MODE]=1;
    } else if !change.full {
        for component in 0..COMPONENTS {
            if mask&(1<<component)==0 || (change.built<N&&RECIPE_COMPONENT[change.built]==component){continue;}
            let stocked=(0..RAW).any(|j|stock[j]>0&&RESOURCE_COMPONENT[RAW_RESOURCES[j]]==component)||(0..R).any(|r|state[STOCK+r]>0&&RESOURCE_COMPONENT[r]==component);
            if stocked||(0..R).any(|r|cap_full(state,r)&&RESOURCE_COMPONENT[r]==component){continue;}
            let saturated=(0..N).filter(|&i|RECIPE_COMPONENT[i]==component).all(|i|output.gross[i]==effective(state[i],change.previous));
            if !saturated{continue;}
            let feasible=(0..R).filter(|&r|RESOURCE_COMPONENT[r]==component).all(|r|RESOURCE_PRODUCERS[r].iter().map(|&i|cap[i]).sum::<u64>()>=RESOURCE_CONSUMERS[r].iter().map(|&i|cap[i]).sum::<u64>());
            if !feasible{continue;}
            for i in 0..N{if RECIPE_COMPONENT[i]==component{if output.gross[i]!=cap[i]{state[GROUP_ACTION+i]=3;}output.gross[i]=cap[i];}}
            for r in 0..R{if RESOURCE_COMPONENT[r]==component{output.balance[r]=RESOURCE_PRODUCERS[r].iter().map(|&i|cap[i] as i64).sum::<i64>()-RESOURCE_CONSUMERS[r].iter().map(|&i|cap[i] as i64).sum::<i64>();}}
            mask&=!(1<<component);scaled+=1;
        }
        state[MODE]=if scaled>0{3}else{2};
    }else{state[MODE]=0;}
    profile::end(9,dispatch_stamp);
    if change.full {
        if use_stock_solver(state){let inventory=stocks(state);let meta=unsafe{&*(p.add(METADATA) as *const active::StockMetadata)};active::solve_stock(&[true;N],&[true;R],&cap,&inventory,output,meta)?;}else{solve_selected(mask,&cap,stock,output)?;}
        state[GROUP_ACTION..GROUP_ACTION+N].fill(1);
    }else if mask!=0 {
        solve_dirty(state,&cap,&change,mask)?;
    }else{output.stages=0;}
    let output=unsafe{&mut *(p.add(INPUT) as *mut Solution)};
    let stamp=profile::start();
    for j in 0..RAW{let r=RAW_RESOURCES[j];output.gross[RAW_RECIPES[j]]+=reserved[j];output.balance[r]+=reserved[j] as i64-state[CONSUMED+r] as i64;}
    for &i in &RAW_RECIPES {let r=RECIPE_RESOURCE[i];if cap_full(state,r)&&output.balance[r]>0{let surplus=output.balance[r] as u64;output.gross[i]=output.gross[i].checked_sub(surplus).ok_or(())?;output.balance[r]=0;}}
    for r in 0..R{if !RAW_RESOURCES.contains(&r)&&state[CONSUMPTION+r]>0{let demand=state[CONSUMPTION+r];let consumed=if state[stock_index(r)]>0{demand}else{demand.min(output.balance[r].max(0) as u64)};state[CONSUMED+r]=consumed;output.balance[r]-=consumed as i64;}}
    let mut next=u64::MAX;
    for r in 0..R{let v=output.balance[r];if v>0&&state[CAPS+r]>0{let remaining=(state[CAPS+r]-1).saturating_sub(state[stock_index(r)]);if remaining==0{return Err(());}next=next.min(mul_div(remaining,Q,v as u64));}let draw=if v<0{(-v) as u64}else{0};state[DRAW+r]=draw;if draw>0{let amount=state[stock_index(r)];if amount==0{return Err(());}next=next.min(mul_div(amount,Q,draw));}}
    output.next=next;
    profile::end(8,stamp);
    state[READY]=1;
    Ok(())
}

#[inline(never)]
fn solve_dirty(state:&mut [u64;WORDS],cap:&[u64;N],change:&Change,mask:u64)->Result<(),()> {
    let stamp=profile::start();
    let mut selected=[false;N];let mut touched=[false;R];
    let mut queue=[0u8;N];let mut length=0;
    for i in 0..N {
        if mask&(1<<RECIPE_COMPONENT[i])==0{continue;}
        let old=state[i]-if i==change.built{Q}else{0};
        if i==change.built || effective(old,change.previous)!=cap[i] {
            selected[i]=true;queue[length]=i as u8;length+=1;
        }
    }
    // Each active group and resource is visited once, including competing producers.
    let mut cursor=0;
    while cursor<length {
        let i=queue[cursor] as usize;cursor+=1;
        for &resource in RECIPE_RESOURCES[i] {
            if touched[resource]{continue;}
            touched[resource]=true;
            for &other in RESOURCE_USERS[resource] {
                if selected[other] || state[other]==0{continue;}
                selected[other]=true;queue[length]=other as u8;length+=1;
            }
        }
    }
    let mut local=[0;N];
    for i in 0..N{if selected[i]{local[i]=cap[i];state[GROUP_ACTION+i]=1;}}
    let p=state.as_mut_ptr();
    let stock=unsafe{&*(p.add(N) as *const [u64;RAW])};
    let output=unsafe{&mut *(p.add(INPUT) as *mut Solution)};
    profile::end(2,stamp);
    if use_stock_solver(state){let inventory=stocks(state);let meta=unsafe{&*(p.add(METADATA) as *const active::StockMetadata)};active::solve_stock(&selected,&touched,&local,&inventory,output,meta)}else{active::solve_active(&selected,&touched,&local,stock,output)}
}

pub mod population;
pub mod colony;

pub mod colony_buildings;
pub mod colony_needs;

pub mod colony_power;
pub mod colony_progression;
