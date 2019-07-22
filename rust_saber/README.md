# rust-saber

<!-- cargo-sync-readme start -->

rust-saber allows writing mods for the Oculus Quest version of Beat Saber in Rust.

# Examples
```rust,no_run
#[repr(C)]
#[derive(Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[rust_saber::hook(0x12DC59C, "example")]
pub unsafe fn get_color(orig: GetColorFn, this: *mut std::ffi::c_void) -> Color {
    let orig_color = unsafe { orig(this) };
    Color {
        r: 1.0,
        g: orig_color.g,
        b: orig_color.b,
        a: orig_color.a,
    }
}
```

The full version of this example can be found at
<https://github.com/leo60228/rust-saber/tree/master/sample_mod>.

<!-- cargo-sync-readme end -->
