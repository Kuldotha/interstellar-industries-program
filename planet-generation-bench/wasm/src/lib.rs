#![allow(dead_code)]
include!("../../common.rs");
static mut TOPOLOGY: [u8; 92240] = [0; 92240];
static mut STATE: [u8; 272] = [0; 272];
#[no_mangle]
pub unsafe extern "C" fn topology_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(TOPOLOGY).cast()
}
#[no_mangle]
pub unsafe extern "C" fn state_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(STATE).cast()
}
#[no_mangle]
pub unsafe extern "C" fn initialize(seed: u32) {
    on_demand::initialize(&mut *core::ptr::addr_of_mut!(STATE), seed);
}
#[no_mangle]
pub unsafe extern "C" fn query(id: u32, side: i32) {
    on_demand::query(
        &*core::ptr::addr_of!(TOPOLOGY),
        &mut *core::ptr::addr_of_mut!(STATE),
        id as usize,
        if side < 0 { None } else { Some(side as usize) },
    );
}
