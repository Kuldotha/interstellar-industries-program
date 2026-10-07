#![allow(dead_code,unused_variables)]
include!("shared.rs");
mod baseline {include!("BASELINE_SHARED");}
fn process(state:&mut[u64;WORDS])->Result<(),()>{prepare_stock(state)?;let change=prepare(state,255)?;execute(state,change)}
fn reference(state:&mut[u64;WORDS])->Result<(),()>{baseline::prepare_stock(state)?;let change=baseline::prepare(state,255)?;baseline::execute(state,change)}
fn unused(s:&[u64;WORDS])->Option<(usize,u64)>{
 for i in 0..N{if RECIPE_RESOURCES[i].len()==1{continue;}let mut amount=effective(s[i],s[FACTOR]).saturating_sub(s[INPUT+i]);for &r in &RECIPE_RESOURCES[i][1..]{if s[stock_index(r)]==0{amount=amount.min((s[INPUT+N+r]as i64).max(0)as u64);}}if amount>Q/100{return Some((i,amount));}}None
}
fn fails(initial:&[u64;WORDS])->bool{let(mut a,mut b)=(*initial,*initial);process(&mut a).is_ok()&&reference(&mut b).is_ok()&&unused(&a).is_some()&&unused(&b).is_none()}
fn main(){
    let mut output=String::new();let mut cases=Vec::new();
    let mut state=[0u64;WORDS];state[AVAILABLE]=65536*Q;state[30]=10*Q;state[31]=Q;state[32]=5*Q;state[51]=10*Q;state[77]=10*Q;
    cases.push(("known local bottleneck".to_owned(),state));
    for(line_id,line)in include_str!("FIXTURES_PATH").lines().enumerate(){let mut state=[0;WORDS];state[AVAILABLE]=65536*Q;for(i,x)in line.split_whitespace().take(INPUT).enumerate(){state[i]=x.parse().unwrap();}cases.push((format!("fixture {line_id}"),state));}
    let mut seed=91u32;for test in 0..1000{let mut state=[0;WORDS];state[AVAILABLE]=65536*Q;for i in 0..N{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);state[i]=((seed>>16)%5)as u64*Q;}if test%2==1{for r in 0..R{seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);if(seed>>16)%3==0{state[stock_index(r)]=10*Q;}}}cases.push((format!("mixed {test}"),state));}
    let(mut matched,mut different,mut rejected,mut stranded)=(0,0,0,0);let mut examples=0;let mut failure=None;let mut max_delta=0u64;
    for(name,initial)in &cases{let(mut candidate,mut original)=(*initial,*initial);reference(&mut original).expect("baseline rejected");if process(&mut candidate).is_err(){rejected+=1;continue;}
        for i in INPUT..INPUT+N{max_delta=max_delta.max(candidate[i].abs_diff(original[i]));}
        let same=(INPUT..INPUT+N).all(|i|candidate[i].abs_diff(original[i])<=128);if same{matched+=1;}else{different+=1;}
        let mut unused=None;
        for i in 0..N {if RECIPE_RESOURCES[i].len()==1{continue;}let cap=effective(candidate[i],candidate[FACTOR]);let mut possible=cap.saturating_sub(candidate[INPUT+i]);
            for &r in &RECIPE_RESOURCES[i][1..]{if initial[stock_index(r)]==0{let surplus=candidate[INPUT+N+r]as i64;possible=possible.min(surplus.max(0)as u64);}}
            if possible>Q/100{unused=Some((i,possible));break;}
        }
        if let Some((i,amount))=unused{stranded+=1;if failure.is_none()&&fails(initial){failure=Some(*initial);}if examples<0{examples+=1;output+=&format!("\n{}: recipe {} can increase by {:.6}/min without reducing another recipe\n",name,i,amount as f64/Q as f64);output+=&format!("capacities {:?}\nstocks {:?}\ncandidate {:?}\nbaseline {:?}\n",initial[..N].iter().enumerate().filter(|(_,x)|**x>0).map(|(i,x)|(i,*x as f64/Q as f64)).collect::<Vec<_>>(),(0..R).filter(|&r|initial[stock_index(r)]>0).collect::<Vec<_>>(),candidate[INPUT..INPUT+N].iter().map(|x|*x as f64/Q as f64).collect::<Vec<_>>(),original[INPUT..INPUT+N].iter().map(|x|*x as f64/Q as f64).collect::<Vec<_>>());}}
        if name=="known local bottleneck"{assert_eq!(candidate[INPUT+32],Q);assert_eq!(candidate[INPUT+77],9*Q);output+=&format!("Known local bottleneck: electronics=1, superconductors=9; passed\n");}
    }
    if let Some(mut small)=failure{
      for r in 0..R{let mut trial=small;trial[stock_index(r)]=0;if fails(&trial){small=trial;}}
      for _ in 0..N{let mut changed=false;for i in 0..N{if small[i]==0{continue;}let mut trial=small;trial[i]=0;if fails(&trial){small=trial;changed=true;}}if !changed{break;}}
      for i in 0..N{for count in 1..small[i]/Q{let mut trial=small;trial[i]=count*Q;if fails(&trial){small=trial;break;}}}
      let(mut a,mut b)=(small,small);process(&mut a).unwrap();reference(&mut b).unwrap();
      output+=&format!("\nReduced failure: {:?}\n",unused(&a));
      for i in 0..N{if small[i]>0{output+=&format!("recipe {i}: capacity={} candidate={} baseline={}\n",small[i]as f64/Q as f64,a[INPUT+i]as f64/Q as f64,b[INPUT+i]as f64/Q as f64);}}
      for r in 0..R{if small[stock_index(r)]>0||a[INPUT+N+r]!=0{output+=&format!("resource {r}: stock={} candidate balance={} baseline balance={}\n",small[stock_index(r)]as f64/Q as f64,(a[INPUT+N+r]as i64)as f64/Q as f64,(b[INPUT+N+r]as i64)as f64/Q as f64);}}
    }
    output+=&format!("\ncases={} matched={} different={} rejected={} directly_improvable={} max_rate_delta_q24={}\n",cases.len(),matched,different,rejected,stranded,max_delta);print!("{output}");std::fs::write(std::env::args().nth(1).unwrap(),output).unwrap();
}
