#[repr(C)]
#[derive(Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[rust_saber::hook(0x12DC59C, "sample_mod")]
pub fn get_color(orig: GetColorFn, this: *mut std::ffi::c_void) -> Color {
    let orig_color = unsafe { orig(this) };
    Color {
        r: 1.0,
        g: orig_color.g,
        b: orig_color.b,
        a: orig_color.a,
    }
}
