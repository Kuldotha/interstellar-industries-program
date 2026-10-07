#![allow(dead_code)]
use std::collections::BTreeMap;
#[derive(Clone,Copy)] struct Row {a:&'static [(usize,i64)],b:&'static [(usize,i64)]}
struct Route {variable:usize,recipe:usize,positive:&'static [Row],negative:&'static [Row],demand:&'static [(usize,i64)],producers:&'static [usize]}
include!("../data.rs");
include!("../components.rs");
type Terms=BTreeMap<(usize,u64),i64>;
#[derive(Clone,PartialEq,Eq)] struct Formula {a:Terms,b:BTreeMap<usize,i64>,full:u64}
fn add<K:Ord+Copy>(a:&mut BTreeMap<K,i64>,b:&BTreeMap<K,i64>,scale:i64){for (&k,&v) in b {let x=a.entry(k).or_default();*x+=v*scale;if *x==0{a.remove(&k);}}}
fn expression(i:usize,all:&mut [Option<Terms>])->Terms {
 if let Some(x)=&all[i]{return x.clone();}
 let r=RECIPE_RESOURCE[i];let mut a=Terms::new();
 if i!=RESOURCE_PRODUCERS[r][0]{a.insert((ROUTES.iter().find(|x|x.recipe==i).unwrap().variable,0),1);}
 else {
  let var=OUTPUT_PRODUCERS.iter().position(|p|p.contains(&i));
  if let Some(v)=var {a.insert((v,0),1);}
  for &c in RESOURCE_CONSUMERS[r]{for ((v,m),k) in expression(c,all){*a.entry((v,m|var.map(|v|1u64<<v).unwrap_or(0))).or_default()+=k;}}
  for &other in &RESOURCE_PRODUCERS[r][1..]{add(&mut a,&expression(other,all),-1);}
 }
 a.retain(|_,v|*v!=0);all[i]=Some(a.clone());a
}
fn useful(f:&Formula)->bool{f.a.values().any(|&v|v>0)||f.b.values().any(|&v|v<0)}
fn push(rows:&mut Vec<Formula>,f:Formula){if useful(&f)&&!rows.contains(&f){rows.push(f);}}
fn terms(t:&Terms)->String{format!("&[{}]",t.iter().map(|(&(v,m),c)|format!("({v},{c},{m})")).collect::<Vec<_>>().join(","))}
fn row(f:&Formula)->String{format!("StockRow{{full:{},a:{},b:&{:?}}}",f.full,terms(&f.a),f.b.iter().map(|(&i,&v)|(i,v)).collect::<Vec<_>>())}
fn main(){
 let mut expr=vec![None;N];for i in 0..N{expression(i,&mut expr);}let expr:Vec<_>=expr.into_iter().map(Option::unwrap).collect();
 let variant_bits=expr.iter().flat_map(|x|x.keys()).filter(|(v,_)|*v>=O).fold(0,|m,(_,b)|m|b);
 let bits:Vec<_>=(0..O).filter(|v|variant_bits&(1<<v)!=0).collect();assert!(bits.len()<=4);
 let mut output=format!("const STOCK_VARIANT_BITS:u64={variant_bits};\nstatic STOCK_EXPRESSIONS:&[&[(usize,i64,u64)]]=&[{}];\n",expr.iter().map(terms).collect::<Vec<_>>().join(","));
 let mut plans=Vec::new();let mut maximum=0;
 for variant in 0..1usize<<bits.len(){
  let mask=bits.iter().enumerate().fold(0u64,|m,(i,b)|m|if variant&(1<<i)!=0{1<<b}else{0});
  let mut rows=Vec::new();for (i,e) in expr.iter().enumerate(){let mut a=Terms::new();for (&(v,m),&c) in e{if m&mask==0{*a.entry((v,m&!variant_bits)).or_default()+=c;}}a.retain(|_,v|*v!=0);push(&mut rows,Formula{full:0,a:a.clone(),b:BTreeMap::from([(i,1)])});push(&mut rows,Formula{full:0,a:a.iter().map(|(&k,&v)|(k,-v)).collect(),b:BTreeMap::new()});}
  for v in 0..O {
   let r=RECIPE_RESOURCE[OUTPUT_PRODUCERS[v][0]];let mut net=Terms::new();
   for &i in RESOURCE_PRODUCERS[r]{add(&mut net,&expr[i],1);}
   for &i in RESOURCE_CONSUMERS[r]{add(&mut net,&expr[i],-1);}
   let mut a=Terms::new();for ((i,m),c) in net{if m&mask==0{*a.entry((i,m&!variant_bits)).or_default()+=c;}}
   a.retain(|_,v|*v!=0);push(&mut rows,Formula{a,b:BTreeMap::new(),full:1<<v});
  }
  let mut routes=Vec::new();
  for variable in O..O+ROUTES.len(){
   let mut positive=Vec::new();let mut negative=Vec::new();rows.retain(|r|{let c=*r.a.get(&(variable,0)).unwrap_or(&0);if c>0{positive.push(r.clone());}else if c<0{negative.push(r.clone());}c==0});
   let lower=Formula{full:0,a:BTreeMap::from([((variable,0),-1)]),b:BTreeMap::new()};if !negative.contains(&lower){negative.push(lower);}
   for p in &positive{for n in &negative{let pc=p.a[&(variable,0)];let nc=-n.a[&(variable,0)];let mut f=Formula{full:0,a:Terms::new(),b:BTreeMap::new()};f.full=p.full|n.full;add(&mut f.a,&p.a,nc);add(&mut f.a,&n.a,pc);add(&mut f.b,&p.b,nc);add(&mut f.b,&n.b,pc);push(&mut rows,f);}}
   let route=ROUTES.iter().find(|r|r.variable==variable).unwrap();
   let mut demand=Terms::new();for &i in route.producers{add(&mut demand,&expr[i],1);}
   routes.push(format!("StockRoute{{variable:{variable},recipe:{},positive:&[{}],negative:&[{}],demand:{},producers:&{:?}}}",route.recipe,positive.iter().map(row).collect::<Vec<_>>().join(","),negative.iter().map(row).collect::<Vec<_>>().join(","),terms(&demand),route.producers));
  }
  maximum=maximum.max(rows.len());let columns=(0..O).map(|v|format!("&[{}]",rows.iter().enumerate().filter_map(|(j,r)|{let terms:Vec<_>=r.a.iter().filter(|(&(i,_),_)|i==v).map(|(&(_,m),&c)|(c,m)).collect();if terms.is_empty(){None}else{Some(format!("StockColumn{{row:{j},terms:&{terms:?}}}"))}}).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(",");
  plans.push(format!("StockPlan{{mask:{mask},rows:&[{}],routes:&[{}],columns:&[{columns}]}}",rows.iter().map(row).collect::<Vec<_>>().join(","),routes.into_iter().rev().collect::<Vec<_>>().join(",")));
 }
 output.push_str(&format!("const STOCK_ROWS:usize={maximum};\nstatic STOCK_PLANS:&[StockPlan]=&[{}];\n",plans.join(",")));
 std::fs::write(std::env::args().nth(1).expect("output path"),output).unwrap();
 println!("Stock metadata: {} fixed variants, {maximum} rows maximum",plans.len());
}
