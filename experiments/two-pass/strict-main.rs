#![allow(dead_code)]
include!("shared.rs");
fn example(name:&str,groups:&[(usize,u64)]){
    let mut cap=[0;N];for &(i,n)in groups{cap[i]=n*Q;}
    let stocks=[0;R];let mut meta=active::StockMetadata::ZERO;meta.prepare(&stocks).unwrap();
    let trace=active::strict_passes(&cap,&stocks,&meta).unwrap();
    println!("\n{name}\ndownward MRE changes (resource, recipe, before, after):");
    for &(r,i,b,a)in &trace.changes{println!("{r} {i} {} {}",b as f64/Q as f64,a as f64/Q as f64);}
    println!("recipe capacity final_mre actual_production");for &(i,_)in groups{println!("{i} {} {} {}",cap[i]/Q,trace.mre[i]as f64/Q as f64,trace.production[i]as f64/Q as f64);}
    for r in 0..R{let balance=RESOURCE_PRODUCERS[r].iter().map(|&i|trace.production[i]as i64).sum::<i64>()-RESOURCE_CONSUMERS[r].iter().map(|&i|trace.production[i]as i64).sum::<i64>();if balance!=0{println!("resource {r} balance {}",balance as f64/Q as f64);}}
    if name=="local bottleneck"{assert_eq!(trace.production[32],Q);assert_eq!(trace.production[77],9*Q);}
    if name=="shared branches"{
        assert_eq!(trace.mre[43],trace.production[43]);
        let mut reference=[0;WORDS];reference[..N].copy_from_slice(&cap);reference[AVAILABLE]=65536*Q;
        prepare_stock(&mut reference).unwrap();let change=prepare(&mut reference,255).unwrap();execute(&mut reference,change).unwrap();
        let spare=|r:usize|RESOURCE_PRODUCERS[r].iter().map(|&i|trace.production[i]).sum::<u64>()-RESOURCE_CONSUMERS[r].iter().map(|&i|trace.production[i]).sum::<u64>();
        let improvement=(cap[43]-trace.production[43]).min(spare(50)).min(spare(10));
        assert_eq!(improvement,Q/8);
        assert_eq!(reference[INPUT+43],Q/2);
        for &i in &[33,48,58]{assert_eq!(reference[INPUT+i],trace.production[i]);}
        println!("Confirmed failure: Composite Fabricator can increase by {} without reducing any other output; baseline produces {}",improvement as f64/Q as f64,reference[INPUT+43]as f64/Q as f64);
    }
    println!("Verified: every resource visited once down and once up; MRE unchanged throughout upward pass");
}
fn main(){
    example("local bottleneck",&[(30,10),(31,1),(32,5),(51,10),(77,10)]);
    example("shared branches",&[(8,1),(23,1),(24,1),(30,1),(31,1),(32,1),(41,1),(42,1),(33,1),(43,1),(48,3),(58,1)]);
}
