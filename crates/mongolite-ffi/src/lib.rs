use std::ffi::c_int;

#[no_mangle]
pub extern "C" fn mongolite_open(_path: *const i8) -> c_int {
    0
}
