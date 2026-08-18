fn main() {
    compile_c_sim();
    linker_be_nice();
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn find_xtensa_gcc() -> std::path::PathBuf {
    if let Ok(cc) = std::env::var("CC") {
        let p = std::path::PathBuf::from(&cc);
        if p.exists() {
            return p;
        }
    }
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap();
    let p = std::path::PathBuf::from(home)
        .join(".rustup/toolchains/esp/xtensa-esp-elf/bin/xtensa-esp32s3-elf-gcc.exe");
    if p.exists() {
        return p;
    }
    std::path::PathBuf::from("xtensa-esp32s3-elf-gcc")
}

fn compile_c_sim() {
    println!("cargo:rerun-if-changed=native/config.h");
    println!("cargo:rerun-if-changed=native/sim.c");
    println!("cargo:rerun-if-changed=native/imu.c");
    println!("cargo:rerun-if-changed=native/lock.c");
    println!("cargo:rerun-if-changed=native/sim.h");
    let mut b = cc::Build::new();
    b.compiler(find_xtensa_gcc());
    b.files(["native/sim.c", "native/imu.c", "native/lock.c"]);
    b.include("native");
    b.flag("-O2");
    b.flag("-ffast-math");
    b.flag("-mlongcalls");
    b.flag("-ffunction-sections");
    b.flag("-fdata-sections");
    b.warnings(false);
    b.compile("fluid_sim");
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
