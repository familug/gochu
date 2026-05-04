fn main() {
    let fcitx5 = pkg_config::Config::new()
        .atleast_version("5.0")
        .probe("Fcitx5Core")
        .unwrap_or_else(|_| {
            eprintln!();
            eprintln!("ERROR: Fcitx5Core not found via pkg-config.");
            eprintln!("Install the fcitx5 development headers:");
            eprintln!("  Arch / Manjaro:  sudo pacman -S fcitx5");
            eprintln!("  Ubuntu / Debian: sudo apt install fcitx5-dev");
            eprintln!();
            std::process::exit(1);
        });

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file("shim/shim.cpp");

    for path in &fcitx5.include_paths {
        build.include(path);
    }

    build.compile("gochu_fcitx5_shim");

    for lib in &fcitx5.libs {
        println!("cargo:rustc-link-lib={lib}");
    }
    for path in &fcitx5.link_paths {
        println!("cargo:rustc-link-search={}", path.display());
    }

    println!("cargo:rerun-if-changed=shim/shim.cpp");
}
