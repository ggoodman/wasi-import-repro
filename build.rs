use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

// A lot of this logic is lifted from https://github.com/bytecodealliance/javy/blob/61616e1507d2bf896f46dc8d72687273438b58b2/crates/quickjs-wasm-sys/build.rs#L18

const WASI_SDK_VERSION_MAJOR: usize = 20;
const WASI_SDK_VERSION_MINOR: usize = 0;

fn download_wasi_sdk() -> PathBuf {
    let mut wasi_sdk_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    wasi_sdk_dir.push("wasi-sdk");

    fs::create_dir_all(&wasi_sdk_dir).unwrap();

    let major_version = WASI_SDK_VERSION_MAJOR;
    let minor_version = WASI_SDK_VERSION_MINOR;

    let mut archive_path = wasi_sdk_dir.clone();
    archive_path.push(format!("wasi-sdk-{major_version}-{minor_version}.tar.gz"));

    println!("SDK tar: {archive_path:?}");

    // Download archive if necessary
    if !archive_path.try_exists().unwrap() {
        let file_suffix = match (env::consts::OS, env::consts::ARCH) {
            ("linux", "x86") | ("linux", "x86_64") => "linux",
            ("macos", "x86") | ("macos", "x86_64") | ("macos", "aarch64") => "macos",
            ("windows", "x86") => "mingw-x86",
            ("windows", "x86_64") => "mingw",
            other => panic!("Unsupported platform tuple {:?}", other),
        };

        let uri = format!("https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-{major_version}/wasi-sdk-{major_version}.{minor_version}-{file_suffix}.tar.gz");

        println!("Downloading WASI SDK archive from {uri} to {archive_path:?}");

        let output = process::Command::new("curl")
            .args([
                "--location",
                "-o",
                archive_path.to_string_lossy().as_ref(),
                uri.as_ref(),
            ])
            .output()
            .unwrap();
        println!("curl output: {}", String::from_utf8_lossy(&output.stdout));
        println!("curl err: {}", String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            panic!(
                "curl WASI SDK failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    let mut test_binary = wasi_sdk_dir.clone();
    test_binary.extend(["bin", "wasm-ld"]);
    // Extract archive if necessary
    if !test_binary.try_exists().unwrap() {
        println!("Extracting WASI SDK archive {archive_path:?}");
        let output = process::Command::new("tar")
            .args([
                "-zxf",
                archive_path.to_string_lossy().as_ref(),
                "--strip-components",
                "1",
            ])
            .current_dir(&wasi_sdk_dir)
            .output()
            .unwrap();
        if !output.status.success() {
            panic!(
                "Unpacking WASI SDK failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    wasi_sdk_dir
}

fn get_wasi_sdk_path() -> PathBuf {
    std::env::var_os("WASI_SDK")
        .map(PathBuf::from)
        .unwrap_or_else(download_wasi_sdk)
}

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "wasi" {
        let wasi_sdk_path = get_wasi_sdk_path();
        if !wasi_sdk_path.try_exists().unwrap() {
            panic!(
                "wasi-sdk not installed in specified path of {}",
                wasi_sdk_path.display()
            );
        }
        env::set_var("WASI_SDK", wasi_sdk_path.to_str().unwrap());
        env::set_var("CC", wasi_sdk_path.join("bin/clang").to_str().unwrap());
        env::set_var("AR", wasi_sdk_path.join("bin/ar").to_str().unwrap());

        println!(
            "cargo:rerun-if-changed={wasi_sdk_path}",
            wasi_sdk_path = wasi_sdk_path.display()
        );
        println!(
            "cargo:rerun-if-changed={}/share/wasi-sysroot/lib/wasm32-wasi/libc.a",
            wasi_sdk_path.display()
        );

        let sysroot = format!(
            "--sysroot={}",
            wasi_sdk_path.join("share/wasi-sysroot").display()
        );
        env::set_var("CFLAGS", &sysroot);

        // Point rust linker to the wasi shared libraries
        println!(
            "cargo:rustc-link-search={}",
            wasi_sdk_path
                .join("share/wasi-sysroot/lib/wasm32-wasi")
                .display()
        );

        let src_dir = Path::new("lib");
        let out_dir = env::var("OUT_DIR").expect("No OUT_DIR env var is set by cargo");
        let out_dir = Path::new(&out_dir);

        let header_files = ["repro.h"];
        let source_files = ["repro.c"];
        for file in source_files.iter().chain(header_files.iter()) {
            fs::copy(src_dir.join(file), out_dir.join(file)).expect("Unable to copy source");
        }

        println!("cargo:rerun-if-env-changed=NO_PRINTF");

        if let Ok(_) = env::var("NO_PRINTF") {
            fs::copy(src_dir.join("repro_no_printf.c"), out_dir.join("repro.c"))
                .expect("Unable to copy repro_no_print.c");
        }

        // The bindgen::Builder is the main entry point
        // to bindgen, and lets you build up options for
        // the resulting bindings.
        let bindings = bindgen::Builder::default()
            .detect_include_paths(true)
            .clang_arg("-xc")
            .clang_arg("-v")
            .clang_arg(format!("--target={}", env::var("TARGET").unwrap()))
            .clang_arg(format!(
                "--sysroot={}",
                wasi_sdk_path.join("share/wasi-sysroot").display()
            ))
            .size_t_is_usize(false)
            .header(out_dir.join("repro.h").display().to_string())
            .allowlist_function("Fallible_func")
            .allowlist_function("Print")
            .clang_arg("-fvisibility=default")
            // The input header we would like to generate
            // bindings for.
            // Tell cargo to invalidate the built crate whenever any of the
            // included header files changed.
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            // Finish the builder and generate the bindings.
            .generate()
            // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings!");

        cc::Build::new()
            .files(source_files.iter().map(|f| out_dir.join(f)))
            .define("_GNU_SOURCE".into(), None)
            .define("CONFIG_VERSION".into(), Some("\"2020-01-19\""))
            .define("CONFIG_BIGNUM".into(), None)
            .define("EMSCRIPTEN".into(), Some("1"))
            .define("FE_DOWNWARD".into(), Some("0"))
            .define("FE_UPWARD".into(), Some("0"))
            .define("NDEBUG".into(), Some("true"))
            .include(out_dir)
            .extra_warnings(false)
            .flag_if_supported("-Wno-implicit-const-int-float-conversion")
            .compile("librepro.a");
    }
}
