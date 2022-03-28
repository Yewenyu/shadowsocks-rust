use std::{ffi::CStr, os::raw::c_char};

#[no_mangle]
pub extern "C" fn ss_client_run(config_path: *const c_char, re_start: bool) -> i32 {
    if let Ok(config_path) = unsafe { CStr::from_ptr(config_path).to_str() } {
        ss_client::ss_start(config_path.to_string(), re_start);
    } else {
        return -1;
    }
    return 0;
}
