#[inline(never)]
fn allocate_values<const STOCK:bool>(mask:u64,cap:&[u64;N],stocked:&[bool;N],values:&mut[u64;52],meta:&StockMetadata)->Result<u64,()>{
    let mut residuals=[u64::MAX;STOCK_ROWS];
    if STOCK {for (j,row) in meta.plan().rows.iter().enumerate(){if meta.masks[j]&mask!=0{if let Some(b)=stock_bound(row,cap,stocked){residuals[j]=b;}}}}
    else {for (j,row) in ROWS.iter().enumerate(){if ROW_MASKS[j]&mask!=0{if let Some(b)=bound(row,cap,stocked){residuals[j]=b;}}}}
    let mut batches=0;
    for (batch_id,batch) in BATCHES.iter().enumerate(){
        let mut weights=[0u64;O];let mut relevant=[0u64;O];
        for &v in *batch {
            if mask&(1<<v)==0{continue;}
            let final_output=!OUTPUT_CONSUMERS[v].iter().any(|&i|cap[i]>0);
            if final_output!=(BATCH_DEPTHS[batch_id]==0){continue;}
            weights[v]=OUTPUT_PRODUCERS[v].iter().map(|&i|cap[i]).sum();
            relevant[v]=weights[v];
            for k in 0..column_len::<STOCK>(v,meta){let (j,c)=column::<STOCK>(v,k,meta);if residuals[j]!=u64::MAX{relevant[v]=relevant[v].min(residuals[j]/c as u64);}}
        }
        if weights.iter().all(|&x|x==0){continue;}
        batches+=1;
        let mut constraints=Vec::new();
        for j in 0..if STOCK{meta.plan().rows.len()}else{ROWS.len()}{
            if residuals[j]==u64::MAX{continue;}
            let mut terms=Vec::new();let mut demand=0u64;
            for &v in *batch{if weights[v]==0{continue;}for k in 0..column_len::<STOCK>(v,meta){let (r,c)=column::<STOCK>(v,k,meta);if r==j{terms.push((v,c as u64));demand+=c as u64*relevant[v];break;}}}
            if demand>0{let level=shared_level(&weights,&relevant,&terms,residuals[j]);constraints.push((j,level,terms));}
        }
        constraints.sort_by(|(a,da,_),(b,db,_)|((da.0 as u128)*(db.1 as u128)).cmp(&((db.0 as u128)*(da.1 as u128))).then(a.cmp(b)));
        for (j,_,terms) in constraints {
            cap_shared_input(&weights,&mut relevant,&terms,residuals[j]);
        }
        // Publish the collected limits together; no residual is changed during collection.
        for &v in *batch{if weights[v]==0{continue;}values[v]=relevant[v];for k in 0..column_len::<STOCK>(v,meta){let(j,c)=column::<STOCK>(v,k,meta);if residuals[j]!=u64::MAX{residuals[j]=residuals[j].checked_sub(relevant[v]*c as u64).ok_or(())?;}}}
    }
    Ok(batches)
}
fn cap_shared_input(weights:&[u64;O],relevant:&mut[u64;O],terms:&[(usize,u64)],available:u64){
    let(n,d)=shared_level(weights,relevant,terms,available);
    for &(v,_) in terms{relevant[v]=relevant[v].min(((weights[v]as u128)*(n as u128)/(d as u128))as u64);}
}
fn shared_level(weights:&[u64;O],relevant:&[u64;O],terms:&[(usize,u64)],available:u64)->(u64,u64){
    if terms.iter().map(|&(v,c)|relevant[v]*c).sum::<u64>()<=available{return (1,1);}
    let mut ordered=terms.to_vec();
    ordered.sort_by(|&(a,_),&(b,_)|((relevant[a] as u128)*(weights[b] as u128)).cmp(&((relevant[b] as u128)*(weights[a] as u128))).then(a.cmp(&b)));
    let mut remaining=available;let mut slope=terms.iter().map(|&(v,c)|weights[v]*c).sum::<u64>();
    for &(v,c) in &ordered{
        if (relevant[v] as u128)*(slope as u128)<=(remaining as u128)*(weights[v] as u128){remaining-=relevant[v]*c;slope-=weights[v]*c;}
        else {return (remaining,slope);}
    }
    (1,1)
}
