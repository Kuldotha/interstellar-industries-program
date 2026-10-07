#![allow(unexpected_cfgs)]
#[cfg(feature="cu-profile")]
extern "C"{fn sol_get_return_data(data:*mut u8,len:u64,program:*mut u8)->u64;}
#[inline(always)]
pub fn start()->u64 {
 #[cfg(feature="cu-profile")]
 {unsafe{crate::sol_remaining_compute_units()}}
 #[cfg(not(feature="cu-profile"))]
 {0}
}
#[inline(always)]
pub fn elapsed(started:u64)->u64 {
 #[cfg(feature="cu-profile")]
 {unsafe{crate::sol_remaining_compute_units()}.abs_diff(started)}
 #[cfg(not(feature="cu-profile"))]
 {let _=started;0}
}
#[inline(always)]
pub fn end(section:usize,started:u64){
 #[cfg(feature="cu-profile")]
 {let cost=elapsed(started);let mut sums=totals();sums[section]+=cost;unsafe{crate::sol_set_return_data(sums.as_ptr().cast(),104)};}
 #[cfg(not(feature="cu-profile"))]
 {let _=(section,started);}
}
#[inline(always)]
pub fn allocation(detail:[u64;3]){
 #[cfg(feature="cu-profile")]
 {let mut sums=totals();for i in 0..3{sums[10+i]+=detail[i];}unsafe{crate::sol_set_return_data(sums.as_ptr().cast(),104)};}
 #[cfg(not(feature="cu-profile"))]
 {let _=detail;}
}
#[cfg(feature="cu-profile")]
pub fn totals()->[u64;13]{let mut sums=[0u64;13];let mut program=[0u8;32];unsafe{sol_get_return_data(sums.as_mut_ptr().cast(),104,program.as_mut_ptr());}sums}
