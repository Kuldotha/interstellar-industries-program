#[derive(Clone,Copy)]
struct Row { a:&'static [(usize,i64)], b:&'static [(usize,i64)] }
struct Route { variable:usize, recipe:usize, positive:&'static [Row], negative:&'static [Row], demand:&'static [(usize,i64)], producers:&'static [usize] }
include!("data.rs");
include!("components.rs");
include!("constraint_groups.rs");
pub const Q:u64=1<<24;
pub const MAX:u64=1<<40;
fn dot(t:&[(usize,i64)],v:&[u64])->i64 { t.iter().map(|&(i,a)|a*(v[i] as i64)).sum() }
fn bound(row:&Row,cap:&[u64;N],stocked:&[bool;N])->Option<u64>{
    if row.b.iter().any(|&(i,_)|stocked[i]) {None} else {Some(dot(row.b,cap) as u64)}
}
#[repr(C)]
#[derive(Debug,PartialEq)]
pub struct Solution {pub gross:[u64;N],pub balance:[i64;R],pub next:u64,pub stages:u64}
#[inline(never)]
pub fn solve_selected(mask:u64,cap:&[u64;N],stock:&[u64;RAW],output:&mut Solution)->Result<(),()> {
    let stamp=crate::profile::start();
    if cap.iter().chain(stock.iter()).any(|&v|v>MAX){return Err(());}
    let mut stocked=[false;N];
    for j in 0..RAW {stocked[RAW_RECIPES[j]]=stock[j]>0;}
    let mut values=[0u64;52];
    crate::profile::end(3,stamp);let stamp=crate::profile::start();
    let stages=allocate_values(mask,cap,&stocked,&mut values)?;
    crate::profile::end(4,stamp);let stamp=crate::profile::start();
    for route in ROUTES {
        if mask&(1<<RECIPE_COMPONENT[route.recipe])==0{continue;}
        let mut low=0i64;let mut high=i64::MAX;
        for (positive,rows) in [(true,route.positive),(false,route.negative)]{
            for row in rows{
                if let Some(b)=bound(row,cap,&stocked){
                    let coefficient=row.a.iter().find(|&&(i,_)|i==route.variable).ok_or(())?.1;
                    let numerator=(b as i64)-dot(row.a,&values);
                    let (n,d)=if coefficient>0{(numerator,coefficient)}else{(-numerator,-coefficient)};
                    let floor=n.div_euclid(d);let ceil=floor+if n.rem_euclid(d)>0{1}else{0};
                    if positive{high=high.min(floor);}else{low=low.max(ceil);}
                }
            }
        }
        if low>high{return Err(());}
        let total:u64=route.producers.iter().map(|&i|cap[i]).sum();
        let demand=dot(route.demand,&values);
        let preferred=if total>0{if let Some(v)=(demand as u64).checked_mul(cap[route.recipe]){(v/total) as i64}else{mul_div(demand as u64,cap[route.recipe],total) as i64}}else{0};
        values[route.variable]=preferred.max(low).min(high) as u64;
    }
    crate::profile::end(5,stamp);let stamp=crate::profile::start();
    let gross=&mut output.gross;
    for i in 0..N{if mask&(1<<RECIPE_COMPONENT[i])==0{continue;}let v=dot(EXPRESSIONS[i],&values);if v<0{return Err(());}gross[i]=v as u64;}
    for &i in &RAW_RECIPES{if mask&(1<<RECIPE_COMPONENT[i])!=0{gross[i]=cap[i];}}
    if gross.iter().zip(cap).any(|(&q,&c)|q>c){return Err(());}
    crate::profile::end(6,stamp);let stamp=crate::profile::start();
    let balance=&mut output.balance;let mut next=u64::MAX;
    for i in 0..R{if mask&(1<<RESOURCE_COMPONENT[i])==0{continue;}balance[i]=RESOURCE_PRODUCERS[i].iter().map(|&j|gross[j] as i64).sum::<i64>()-RESOURCE_CONSUMERS[i].iter().map(|&j|gross[j] as i64).sum::<i64>();}
    for j in 0..RAW{let v=balance[RAW_RESOURCES[j]];if v<0{if stock[j]==0{return Err(());}let time=if let Some(amount)=stock[j].checked_mul(Q){amount/((-v) as u64)}else{((stock[j] as u128)*(Q as u128)/((-v) as u128)).min(u64::MAX as u128) as u64};next=next.min(time);}}
    for i in 0..R{if balance[i]<0 && !RAW_RESOURCES.contains(&i){return Err(());}}
    crate::profile::end(7,stamp);
    output.next=next;output.stages=stages;Ok(())
}

#[inline(never)]
pub fn solve(cap:&[u64;N],stock:&[u64;RAW])->Result<Solution,()>{let mut s=Solution{gross:[0;N],balance:[0;R],next:u64::MAX,stages:0};solve_selected((1<<COMPONENTS)-1,cap,stock,&mut s)?;Ok(s)}

#[inline(never)]
fn allocate_values(mask:u64,cap:&[u64;N],stocked:&[bool;N],values:&mut[u64;52])->Result<u64,()>{
    let mut residuals=[u64::MAX;GROUP_COUNT];
    for (i,row) in ROWS.iter().enumerate(){if mask&(1<<ROW_COMPONENT[i])==0{continue;}if let Some(b)=bound(row,cap,stocked){let group=ROW_GROUP[i];residuals[group]=residuals[group].min(b);}}
    let mut stages=0;
    let mut detail=[0u64;3];
    for (batch_id,batch) in BATCHES.iter().enumerate(){
        if mask&(1<<BATCH_COMPONENT[batch_id])==0{continue;}
        let mut weights=[0u64;O];let mut active=0u64;
        let mut slopes=[0u64;GROUP_COUNT];
        for &i in *batch{let final_output=!OUTPUT_CONSUMERS[i].iter().any(|&j|cap[j]>0);if final_output!=(BATCH_DEPTHS[batch_id]==0){continue;}weights[i]=OUTPUT_PRODUCERS[i].iter().map(|&j|cap[j]).sum();if weights[i]>0{active|=1<<i;for k in 0..GROUP_COLUMNS[i].len(){let(r,c)=GROUP_COLUMNS[i][k];slopes[r]+=weights[i]*(c as u64);}}}
        let mut rows=[0u8;GROUP_COUNT];let mut row_count=0;
        for k in 0..GROUP_BATCH_ROWS[batch_id].len(){let j=GROUP_BATCH_ROWS[batch_id][k];if residuals[j]!=u64::MAX&&slopes[j]>0{rows[row_count]=j as u8;row_count+=1;}}
        while active!=0{
            stages+=1;
            let stamp=crate::profile::start();
            let(mut numerator,mut denominator,mut mask)=(1u64,1u64,0u64);
            let mut pending=active;
            while pending!=0{let i=pending.trailing_zeros() as usize;pending&=pending-1;
                let n=weights[i]-values[i];let d=weights[i];let order=compare_products(n,denominator,numerator,d);
                if mask==0 || order<0{numerator=n;denominator=d;mask=1<<i;}else if order==0{mask|=1<<i;}
            }
            let mut k=0;
            while k<row_count{let j=rows[k] as usize;if slopes[j]==0{row_count-=1;rows[k]=rows[row_count];continue;}k+=1;
                let order=compare_products(residuals[j],denominator,numerator,slopes[j]);
                if order<=0{
                    let row_mask=GROUP_MASKS[j];
                    if order<0{numerator=residuals[j];denominator=slopes[j];mask=row_mask;}else{mask|=row_mask;}
                }
            }
            detail[0]+=crate::profile::elapsed(stamp);let stamp=crate::profile::start();
            let mut pending=active;
            while pending!=0{let i=pending.trailing_zeros() as usize;pending&=pending-1;
                let increment=if numerator==denominator{weights[i]}else if numerator==0{0}else if let Some(v)=weights[i].checked_mul(numerator){v/denominator}else{mul_div(weights[i],numerator,denominator)};
                values[i]+=increment;
                for k in 0..GROUP_COLUMNS[i].len(){let(j,c)=GROUP_COLUMNS[i][k];if residuals[j]!=u64::MAX{residuals[j]=residuals[j].checked_sub(increment*(c as u64)).ok_or(())?;}}
            }
            detail[1]+=crate::profile::elapsed(stamp);let stamp=crate::profile::start();
            let fixed=active&mask;
            if fixed==0{return Err(());}
            let mut pending=fixed;
            while pending!=0{let i=pending.trailing_zeros() as usize;pending&=pending-1;for k in 0..GROUP_COLUMNS[i].len(){let(j,c)=GROUP_COLUMNS[i][k];slopes[j]-=weights[i]*(c as u64);}}
            active&=!fixed;
            detail[2]+=crate::profile::elapsed(stamp);
        }
    }
    crate::profile::allocation(detail);
    Ok(stages)
}

#[inline(never)]
fn mul_div(a:u64,n:u64,d:u64)->u64{
    let product=(a as u128)*(n as u128);
    let high=(product>>64) as u64;let low=product as u64;
    let shift=d.leading_zeros();let v=d<<shift;
    let un32=if shift==0{high}else{(high<<shift)|(low>>(64-shift))};
    let un10=low<<shift;let un1=un10>>32;let un0=un10&0xffff_ffff;
    let vn1=v>>32;let vn0=v&0xffff_ffff;let base=1u64<<32;
    let mut q1=un32/vn1;let mut rhat=un32-q1*vn1;
    for _ in 0..2{
        if q1<base && (rhat>=base || q1*vn0<=base*rhat+un1){break;}
        q1-=1;rhat+=vn1;
    }
    let un21=un32.wrapping_mul(base).wrapping_add(un1).wrapping_sub(q1.wrapping_mul(v));
    let mut q0=un21/vn1;rhat=un21-q0*vn1;
    for _ in 0..2{
        if q0<base && (rhat>=base || q0*vn0<=base*rhat+un0){break;}
        q0-=1;rhat+=vn1;
    }
    q1*base+q0
}

#[inline(always)]
fn compare_products(a:u64,b:u64,c:u64,d:u64)->i8{
    if (a|b|c|d)>>32==0 {return if a*b<c*d{-1}else if a*b>c*d{1}else{0};}
    let (ah,bh,ch,dh)=(a>>32,b>>32,c>>32,d>>32);
    let (left,right)=(ah*bh,ch*dh);
    if left+ah+bh+1<right{return -1;}
    if right+ch+dh+1<left{return 1;}
    let (left,right)=((a as u128)*(b as u128),(c as u128)*(d as u128));
    if left<right{-1}else if left>right{1}else{0}
}
