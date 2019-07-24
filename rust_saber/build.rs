use cc;

fn main() {
    if std::env::var("TARGET").unwrap() != "armv7-linux-androideabi" {
        return;
    }

    let mut build = cc::Build::new();
    build.warnings(false);
    build.flag("-Wno-everything");
    build.include("inline-hook");
    build.file("inline-hook/inlineHook.c");
    build.file("inline-hook/relocate.c");
    build.compile("libinlinehook.a");
}
