#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[export_name = "fallible_func"]
pub extern "C" fn fallible_func(val: i32) -> i32 {
    unsafe { Fallible_func(val) }
}

#[cfg(feature = "print")]
#[export_name = "print"]
pub extern "C" fn print(str: *const i8) {
    unsafe {
        // Call the FFI Print function with the a pointer to the C string `cstr`.
        Print(str as *mut i8);
    };
}
