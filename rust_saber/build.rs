use cc;

fn main() {
    let mut build = cc::Build::new();
    build.warnings(false);
    build.include("inline-hook");
    build.file("inline-hook/inlineHook.c");
    build.file("inline-hook/relocate.c");
    build.compile("libinlinehook.a");
}
