#[inline(never)]
fn allocate_values<const STOCK:bool>(mask:u64,cap:&[u64;N],stocked:&[bool;N],values:&mut[u64;52],meta:&StockMetadata)->Result<u64,()>{
    let mut residuals=[u64::MAX;STOCK_ROWS];
    if STOCK {for (j,row) in meta.plan().rows.iter().enumerate(){if meta.masks[j]&mask!=0{if let Some(b)=stock_bound(row,cap,stocked){residuals[j]=b;}}}}
    else {for (j,row) in ROWS.iter().enumerate(){if ROW_MASKS[j]&mask!=0{if let Some(b)=bound(row,cap,stocked){residuals[j]=b;}}}}
    let mut stages=0;
    for (batch_id,batch) in BATCHES.iter().enumerate(){
        let mut weights=[0u64;O];let mut relevant=[0u64;O];let mut active=0u64;
        for &v in *batch {
            if mask&(1<<v)==0{continue;}
            let final_output=!OUTPUT_CONSUMERS[v].iter().any(|&i|cap[i]>0);
            if final_output!=(BATCH_DEPTHS[batch_id]==0){continue;}
            weights[v]=OUTPUT_PRODUCERS[v].iter().map(|&i|cap[i]).sum();
            if weights[v]==0{continue;}active|=1<<v;relevant[v]=weights[v];
            for k in 0..column_len::<STOCK>(v,meta){let (j,c)=column::<STOCK>(v,k,meta);if residuals[j]!=u64::MAX{relevant[v]=relevant[v].min(if c==1{residuals[j]}else{residuals[j]/c as u64});}}
        }
        if active==0{continue;}
        let row_count=if STOCK{meta.plan().rows.len()}else{ROWS.len()};
        let mut pending=0u128;let mut numerator=[1u64;STOCK_ROWS];let mut denominator=[1u64;STOCK_ROWS];
        let mut changed=active;
        for j in 0..row_count{let bits=if STOCK{meta.masks[j]}else{ROW_MASKS[j]};if residuals[j]!=u64::MAX&&bits&active!=0{pending|=1u128<<j;}}
        while pending!=0{
            let mut chosen=usize::MAX;let mut best=(1u64,1u64);let mut bits=pending;
            while bits!=0{let j=bits.trailing_zeros()as usize;bits&=bits-1;let variables=if STOCK{meta.masks[j]}else{ROW_MASKS[j]};
                if variables&changed!=0{let level=shared_level::<STOCK>(j,&weights,&relevant,meta,residuals[j]);numerator[j]=level.0;denominator[j]=level.1;}
                if compare_products(numerator[j],best.1,best.0,denominator[j])<0{chosen=j;best=(numerator[j],denominator[j]);}
            }
            if chosen==usize::MAX{break;}stages+=1;pending&=!(1u128<<chosen);changed=0;
            let mut variables=active&if STOCK{meta.masks[chosen]}else{ROW_MASKS[chosen]};
            while variables!=0{let v=variables.trailing_zeros()as usize;variables&=variables-1;let limit=ratio(weights[v],best.0,best.1);if limit<relevant[v]{relevant[v]=limit;changed|=1<<v;}}
        }
        for &v in *batch{if weights[v]==0{continue;}values[v]=relevant[v];for k in 0..column_len::<STOCK>(v,meta){let(j,c)=column::<STOCK>(v,k,meta);if residuals[j]!=u64::MAX{residuals[j]=residuals[j].checked_sub(relevant[v]*c as u64).ok_or(())?;}}}
    }
    Ok(stages)
}
fn ratio(a:u64,n:u64,d:u64)->u64{if n==d{a}else if let Some(p)=a.checked_mul(n){p/d}else{mul_div(a,n,d)}}
#[inline(never)]
fn shared_level<const STOCK:bool>(j:usize,weights:&[u64;O],relevant:&[u64;O],meta:&StockMetadata,available:u64)->(u64,u64){
    let mut coefficients=[0i16;O];
    if STOCK{for &(v,c,m)in meta.plan().rows[j].a{if m&meta.stocked==0{coefficients[v]+=c as i16;}}}else{for &(v,c)in ROWS[j].a{coefficients[v]=c as i16;}}
    let mut ordered=[0u8;O];let mut count=0;let mut demand=0;let mut slope=0;
    let mut bits=if STOCK{meta.masks[j]}else{ROW_MASKS[j]};
    while bits!=0{let v=bits.trailing_zeros()as usize;bits&=bits-1;if weights[v]==0{continue;}let c=coefficients[v]as u64;demand+=c*relevant[v];slope+=c*weights[v];ordered[count]=v as u8;count+=1;}
    if demand<=available{return(1,1);}
    for at in 1..count{let v=ordered[at]as usize;let mut k=at;while k>0{let other=ordered[k-1]as usize;if compare_products(relevant[v],weights[other],relevant[other],weights[v])>=0{break;}ordered[k]=ordered[k-1];k-=1;}ordered[k]=v as u8;}
    let mut remaining=available;
    for &v in &ordered[..count]{let v=v as usize;if compare_products(relevant[v],slope,remaining,weights[v])<=0{remaining-=relevant[v]*coefficients[v]as u64;slope-=weights[v]*coefficients[v]as u64;}else{return(remaining,slope);}}
    (1,1)
}
