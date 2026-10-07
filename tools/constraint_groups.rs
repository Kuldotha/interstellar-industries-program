#![allow(dead_code)]
use std::collections::BTreeMap;
use std::fmt::Write;
#[derive(Clone,Copy)]
struct Row { a:&'static [(usize,i64)], b:&'static [(usize,i64)] }
struct Route { variable:usize, recipe:usize, positive:&'static [Row], negative:&'static [Row], demand:&'static [(usize,i64)], producers:&'static [usize] }
include!("../data.rs");
struct StockRow {full:u64,a:&'static [(usize,i64,u64)],b:&'static [(usize,i64)]}
struct StockRoute {variable:usize,recipe:usize,positive:&'static [StockRow],negative:&'static [StockRow],demand:&'static [(usize,i64,u64)],producers:&'static [usize]}
struct StockColumn {row:usize,terms:&'static [(i64,u64)]}
struct StockPlan {columns:&'static [&'static [StockColumn]],mask:u64,rows:&'static [StockRow],routes:&'static [StockRoute]}
include!("../stock_data.rs");
fn groups<T:Ord+Clone>(rows:impl Iterator<Item=Vec<T>>) -> (Vec<usize>,Vec<Vec<usize>>) {
    let mut keys=BTreeMap::new();let mut mapping=Vec::new();let mut members:Vec<Vec<usize>>=Vec::new();
    for (i,mut key) in rows.enumerate(){key.sort();let next=keys.len();let group=*keys.entry(key).or_insert(next);if group==members.len(){members.push(Vec::new());}members[group].push(i);mapping.push(group);}
    (mapping,members)
}
fn main(){
    let (mapping,members)=groups(ROWS.iter().map(|r|r.a.to_vec()));
    let mut output=String::new();
    writeln!(output,"const GROUP_COUNT:usize={};",members.len()).unwrap();
    writeln!(output,"static ROW_GROUP:&[usize]=&{mapping:?};").unwrap();
    let masks:Vec<u64>=members.iter().map(|g|ROWS[g[0]].a.iter().fold(0,|mask,&(v,_)|mask|1<<v)).collect();
    writeln!(output,"static GROUP_MASKS:&[u64]=&{masks:?};").unwrap();
    output.push_str("static GROUP_COLUMNS:&[&[(usize,i64)]]=&[");
    for col in COLUMNS{let mut merged=BTreeMap::new();for &(r,c) in *col{assert!(merged.insert(mapping[r],c).is_none_or(|old|old==c));}let column:Vec<_>=merged.into_iter().collect();write!(output,"&{column:?},").unwrap();}output.push_str("];\n");
    output.push_str("static GROUP_BATCH_ROWS:&[&[usize]]=&[");
    for rows in BATCH_ROWS{let mut ids:Vec<_>=rows.iter().map(|&r|mapping[r]).collect();ids.sort();ids.dedup();write!(output,"&{ids:?},").unwrap();}output.push_str("];\n");
    output.push_str("static STOCK_ROW_GROUPS:&[&[usize]]=&[");
    let mut counts=Vec::new();for plan in STOCK_PLANS{let(map,groups)=groups(plan.rows.iter().map(|row|row.a.to_vec()));write!(output,"&{map:?},").unwrap();counts.push(groups.len());}output.push_str("];\n");
    writeln!(output,"static STOCK_GROUP_COUNTS:&[usize]=&{counts:?};").unwrap();
    output.push_str("static STOCK_GROUP_COLUMNS:&[&[&[(usize,&[(i64,u64)])]]]=&[");
    for plan in STOCK_PLANS {
        let(mapping,_)=groups(plan.rows.iter().map(|row|row.a.to_vec()));
        output.push_str("&[");
        for column in plan.columns {
            let mut merged=BTreeMap::new();
            for entry in *column {
                let mut terms=entry.terms.to_vec();terms.sort();
                assert!(merged.insert(mapping[entry.row],terms.clone()).is_none_or(|old|old==terms));
            }
            output.push_str("&[");
            for (group,terms) in merged {write!(output,"({group},&{terms:?}),").unwrap();}
            output.push_str("],");
        }
        output.push_str("],");
    }
    output.push_str("];\n");
    std::fs::write(std::env::args().nth(1).unwrap(),output).unwrap();
    println!("Allocation constraints: {} -> {}; stock variants {:?}",ROWS.len(),members.len(),counts);
}
