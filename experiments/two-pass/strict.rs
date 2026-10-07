#[derive(Clone)]
pub struct StrictTrace {
    pub mre:[u64;84],
    pub production:[u64;84],
    pub down_visits:[u8;80],
    pub up_visits:[u8;80],
    pub changes:Vec<(usize,usize,u64,u64)>,
}
pub fn strict_passes(cap:&[u64;N],stocks:&[u64;R],meta:&StockMetadata)->Result<StrictTrace,()> {
    let mut raw_stock=[false;N];for j in 0..RAW{raw_stock[RAW_RECIPES[j]]=stocks[RAW_RESOURCES[j]]>0;}
    let mut ceiling=*cap;
    // These are independent capacity bounds from the shipped supplier expressions.
    for v in 0..O {
        let mut possible=OUTPUT_PRODUCERS[v].iter().map(|&i|cap[i]).sum::<u64>();
        for col in meta.plan().columns[v] {
            let c:i64=col.terms.iter().filter(|&&(_,m)|m&meta.stocked==0).map(|&(c,_)|c).sum();
            if c>0{if let Some(b)=stock_bound(&meta.plan().rows[col.row],cap,&raw_stock){possible=possible.min(b/c as u64);}}
        }
        for &i in OUTPUT_PRODUCERS[v]{ceiling[i]=ceiling[i].min(possible);}
    }
    let mut order:Vec<usize>=(0..R).collect();order.sort_by_key(|&r|*RESOURCE_PRODUCERS[r].iter().max().unwrap());
    let mut trace=StrictTrace{mre:ceiling,production:[0;N],down_visits:[0;R],up_visits:[0;R],changes:Vec::new()};
    for &r in order.iter().rev(){
        trace.down_visits[r]+=1;
        if stocks[r]>0{continue;}
        let supply=RESOURCE_PRODUCERS[r].iter().map(|&i|ceiling[i]).sum();
        let shares=strict_share(RESOURCE_CONSUMERS[r],cap,&trace.mre,supply);
        for &i in RESOURCE_CONSUMERS[r]{if shares[i]<trace.mre[i]{trace.changes.push((r,i,trace.mre[i],shares[i]));trace.mre[i]=shares[i];}}
    }
    let finalized=trace.mre;
    let mut received=[u64::MAX;N];
    for &r in &order{
        trace.up_visits[r]+=1;
        for &i in RESOURCE_PRODUCERS[r]{trace.production[i]=if RAW_RECIPES.contains(&i){cap[i]}else{finalized[i].min(received[i])};}
        let shares=if stocks[r]>0{finalized}else{strict_share(RESOURCE_CONSUMERS[r],cap,&finalized,RESOURCE_PRODUCERS[r].iter().map(|&i|trace.production[i]).sum())};
        for &i in RESOURCE_CONSUMERS[r]{received[i]=received[i].min(shares[i]);}
    }
    assert_eq!(trace.mre,finalized);
    assert!(trace.down_visits.iter().all(|&n|n==1));assert!(trace.up_visits.iter().all(|&n|n==1));
    Ok(trace)
}
fn strict_share(consumers:&[usize],weights:&[u64;N],limits:&[u64;N],supply:u64)->[u64;N]{
    let mut result=[0;N];let demand=consumers.iter().map(|&i|limits[i]).sum::<u64>();
    if supply>=demand{for &i in consumers{result[i]=limits[i];}return result;}
    let mut sorted:Vec<_>=consumers.iter().copied().filter(|&i|weights[i]>0).collect();
    sorted.sort_by(|&a,&b|((limits[a]as u128)*(weights[b]as u128)).cmp(&((limits[b]as u128)*(weights[a]as u128))));
    let mut remaining=supply;let mut weight=sorted.iter().map(|&i|weights[i]).sum::<u64>();
    for (k,&i)in sorted.iter().enumerate(){
        if(limits[i]as u128)*(weight as u128)<=(remaining as u128)*(weights[i]as u128){result[i]=limits[i];remaining-=limits[i];weight-=weights[i];}
        else{for &j in &sorted[k..]{result[j]=((weights[j]as u128)*(remaining as u128)/(weight as u128))as u64;}break;}
    }result
}
