pub const BUILDING_COUNT:usize=15;
pub const RESOURCE_IDS:[usize;8]=[27,16,23,55,79,75,5,6];
pub const RECIPES:[usize;8]=[0,1,2,8,9,10,11,12];
pub const HOUSE:u8=2;
pub const DOCK:u8=3;
pub const COMMONS:u8=4;
pub const RADIO:u8=11;
pub fn recipe(kind:u8)->Option<usize>{match kind{0=>Some(0),1=>Some(1),3=>Some(2),5=>Some(8),6=>Some(9),7=>Some(10),8=>Some(11),9=>Some(12),_=>None}}
pub fn resource(kind:u8)->Option<usize>{recipe(kind).and_then(|r|RECIPES.iter().position(|&v|v==r))}
pub fn workers(kind:u8)->u64{match kind{0=>2,1=>3,3=>5,5|7|9=>5,6|8|10=>10,_=>0}}
pub fn cost(kind:u8)->u64{match kind{4=>3,5|7|9=>3,6|8=>5,10=>8,11=>6,_=>1}}
pub fn unlock_population(kind:u8)->u64{match kind{3=>4,0|1=>10,4=>20,5..=8|12|13=>60,9..=11|14=>100,_=>0}}
pub fn utility(kind:u8)->bool{kind==COMMONS||kind==RADIO}

pub fn power_demand(kind:u8)->u64{match kind{RADIO=>1,_=>0}}
pub fn field_kind(kind:u8)->Option<u8>{match kind{5=>Some(12),7=>Some(13),9=>Some(14),_=>None}}
pub fn farm_kind(kind:u8)->Option<u8>{match kind{12=>Some(5),13=>Some(7),14=>Some(9),_=>None}}
pub const POWERED_RECIPES:[usize;5]=[0,1,2,9,11];
