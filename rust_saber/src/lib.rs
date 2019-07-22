//! rust-saber allows writing mods for the Oculus Quest version of Beat Saber in Rust.
//!
//! # Examples
//! ```rust,no_run
//! #[repr(C)]
//! #[derive(Default)]
//! pub struct Color {
//!     pub r: f32,
//!     pub g: f32,
//!     pub b: f32,
//!     pub a: f32,
//! }
//!
//! #[rust_saber::hook(0x12DC59C, "example")]
//! pub unsafe fn get_color(orig: GetColorFn, this: *mut std::ffi::c_void) -> Color {
//!     let orig_color = unsafe { orig(this) };
//!     Color {
//!         r: 1.0,
//!         g: orig_color.g,
//!         b: orig_color.b,
//!         a: orig_color.a,
//!     }
//! }
//! ```
//!
//! The full version of this example can be found at
//! <https://github.com/leo60228/rust-saber/tree/master/sample_mod>.

use std::env;
use std::sync::Once;
use std::ffi::CString;
use lazy_static::lazy_static;
use log::*;

#[doc(inline)]
pub use rust_saber_macros::hook;

static INIT: Once = Once::new();

/// This must be called once with the name of your mod by your program for rust-saber to work
/// properly. Currently, this is called automatically by every hook. In the future, there may
/// be more specific guarantees about what happens when this is not called.
pub fn init_once(name: &'static str) {
    INIT.call_once(move || {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag(name)
                .with_min_level(Level::Trace)
        );

        env::set_var("RUST_BACKTRACE", "1");
        log_panics::init();

        info!("{} initialized!", name);
    });
}

lazy_static! {
    static ref BASE_ADDR: u64 = base_addr("/data/app/com.beatgames.beatsaber-1/lib/arm/libil2cpp.so");
}

/// Get the base address of any .so file loaded into the current process.
///
/// # Examples
/// ```rust,no_run
/// # use lazy_static::lazy_static;
/// lazy_static! {
///     static ref BASE_ADDR: u64 = base_addr("/data/app/com.beatgames.beatsaber-1/lib/arm/libil2cpp.so");
/// }
/// ```
///
/// # Panics
/// This function will panic if the .so is not loaded.
pub fn base_addr(so_path: &str) -> u64 {
    let cstring = CString::new(so_path).unwrap();
    let handle = unsafe { libc::dlopen(cstring.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_LAZY) };
    if handle.is_null() {
        0
    } else {
        let maps = proc_maps::get_process_maps(unsafe { libc::getpid() }).unwrap();
        let map = maps.iter().find(|e| e.filename().as_ref().map(|e| e.ends_with(so_path)).unwrap_or(false)).expect("Can't find base address in mappings!");
        map.start() as u64
    }
}

/// Get an address relative to libil2cpp.so.
pub fn bs_offset(offset: u32) -> u64 {
    *BASE_ADDR + (offset as u64)
}

#[repr(C)]
#[allow(dead_code)]
enum Ele7enStatus {
    ErrorUnknown = -1,
    Ok = 0,
    ErrorNotInitialized,
    ErrorNotExecutable,
    ErrorNotRegistered,
    ErrorNotHooked,
    ErrorAlreadyRegistered,
    ErrorAlreadyHooked,
    ErrorSoNotFound,
    ErrorFunctionNotFound,
}

#[cfg(target_os = "android")]
extern {
    fn registerInlineHook(target: u32, new: u32, orig: *mut *mut u32) -> Ele7enStatus;
    fn inlineHook(target: u32) -> Ele7enStatus;
}

/// Hook the function at addr, using the function at func as a replacement. This should not be used
/// directly, use the hook attribute instead.
///
/// # Unsafety
/// This can cause unsafety if either function is not a valid address, or if they have different
/// signatures.
pub unsafe fn hook(func: u32, addr: u32) -> *mut () {
    trace!("0x{:x} -> 0x{:x}", addr, func);
    let mut ptr: *mut u32 = std::ptr::null_mut();
    let offset = bs_offset(addr) as u32;
    #[cfg(target_os = "android")]
    registerInlineHook(offset, func, &mut ptr as *mut *mut u32);
    #[cfg(target_os = "android")]
    inlineHook(offset);
    ptr as *mut ()
}
