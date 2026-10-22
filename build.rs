fn main() {
    // Keep test harnesses runnable without UAC; distributed release binaries require elevation.
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        return;
    }
    println!("cargo:rerun-if-changed=app.manifest");
    match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => {
            println!("cargo:rustc-link-arg-bin=frosthaven-lan=/MANIFEST:EMBED");
            println!(
                "cargo:rustc-link-arg-bin=frosthaven-lan=/MANIFESTINPUT:{}",
                std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
                    .join("app.manifest")
                    .display()
            );
            println!("cargo:rustc-link-arg-bin=frosthaven-lan=/MANIFESTUAC:NO");
        }
        Ok("gnu") => {
            let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
            let resource = out.join("manifest.rc");
            let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("app.manifest");
            std::fs::write(
                &resource,
                format!(
                    "1 24 \"{}\"\n",
                    manifest.display().to_string().replace('\\', "/")
                ),
            )
            .unwrap();
            let object = out.join("manifest.o");
            let status = std::process::Command::new("windres")
                .arg(&resource)
                .arg("-O")
                .arg("coff")
                .arg("-o")
                .arg(&object)
                .status()
                .expect("GNU builds require windres on PATH");
            assert!(status.success(), "Failed to compile administrator manifest");
            println!(
                "cargo:rustc-link-arg-bin=frosthaven-lan={}",
                object.display()
            );
        }
        _ => panic!("Only Windows MSVC and GNU builds are supported"),
    }
}
