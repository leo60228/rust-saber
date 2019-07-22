use std::env;
use std::sync::Once;
use std::ffi::CString;
use lazy_static::lazy_static;
use log::*;

pub use rust_saber_macros::*;

static INIT: Once = Once::new();

pub fn init_once(name: &'static str) {
    INIT.call_once(move || {
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

pub fn base_addr(so_name: &str) -> u64 {
    let cstring = CString::new(so_name).unwrap();
    let handle = unsafe { libc::dlopen(cstring.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_LAZY) };
    if handle.is_null() {
        0
    } else {
        let maps = proc_maps::get_process_maps(unsafe { libc::getpid() }).unwrap();
        let map = maps.iter().find(|e| e.filename().as_ref().map(|e| e.ends_with(so_name)).unwrap_or(false)).expect("Can't find base address in mappings!");
        map.start() as u64
    }
}

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

extern {
    fn registerInlineHook(target: u32, new: u32, orig: *mut *mut u32) -> Ele7enStatus;
    fn inlineHook(target: u32) -> Ele7enStatus;
}

pub unsafe fn hook(func: u32, addr: u32) -> *mut () {
    trace!("0x{:x} -> 0x{:x}", addr, func);
    let mut ptr: *mut u32 = std::ptr::null_mut();
    let offset = bs_offset(addr) as u32;
    registerInlineHook(offset, func, &mut ptr as *mut *mut u32);
    inlineHook(offset);
    ptr as *mut ()
}
