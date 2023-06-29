use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use meson_next::config::Config;

fn main() {
    let build_static = cfg!(feature = "build-static");
    let build_shared = cfg!(feature = "build-shared");

    // 1. Prepare paths
    let libvfio_user_path = PathBuf::from("libvfio-user");
    let libvfio_user_path_str = libvfio_user_path.to_str().unwrap();

    let header_path = libvfio_user_path.join("include/libvfio-user.h");
    let header_path_str = header_path.to_str().unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let bindings_path = out_path.join("bindings.rs");

    let build_path = out_path.join("build");
    let build_path_str = build_path.to_str().unwrap();

    let lib_path = build_path.join("lib");
    let lib_path_str = lib_path.to_str().unwrap();

    // 2. Configure cargo
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", lib_path_str);

    // Tell cargo to tell rustc to link our `vfio-user` library. Cargo will
    // automatically know it must look for a `libvfio-user.a` or `libvfio-user.so` file.

    if build_static {
        // Prefer linking statically when both static and shared libraries are built
        // Look for a `libvfio-user.a` file
        println!("cargo:rustc-link-lib=static=vfio-user");
    } else if build_shared {
        // Look for a `libvfio-user.so` file
        println!("cargo:rustc-link-lib=dylib=vfio-user");
    } else {
        // Look for any kind of `libvfio-user` library
        println!("cargo:rustc-link-lib=vfio-user");
    }

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed={}", header_path_str);

    // 3. Build libvfio-user

    // Try to include dependencies of libvfio-user
    // pkg_config will automatically configure cargo to link the dependencies
    if pkg_config::Config::new()
        .cargo_metadata(true)
        .atleast_version("0.11")
        .probe("json-c")
        .is_err()
    {
        println!("cargo:warning=Could not find json-c >= 0.11, build may fail");
    }

    if pkg_config::Config::new()
        .cargo_metadata(true)
        .probe("cmocka")
        .is_err()
    {
        println!("cargo:warning=Could not find cmocka, build may fail");
    }

    if build_static || build_shared {
        let mut meson_options = HashMap::new();

        if build_static && build_shared {
            meson_options.insert("default_library", "both");
        } else if build_static {
            meson_options.insert("default_library", "static");
        } else {
            meson_options.insert("default_library", "shared");
        }

        let meson_config = Config::new().options(meson_options);
        meson_next::build(libvfio_user_path_str, build_path_str, meson_config);
    }

    // 4. Generate bindings
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(header_path_str)
        .allowlist_file(header_path_str)
        // Parse all comments since some explanations are not doc comments (/* ... */ vs /** ... */)
        .clang_arg("-fparse-all-comments")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(bindings_path)
        .expect("Couldn't write bindings!");
}
