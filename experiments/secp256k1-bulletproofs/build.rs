fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dst = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dst)
        } else {
            std::fs::copy(entry.path(), dst).unwrap();
        }
    }
}
fn main() {
    let source = std::env::var("CNU_BP_SOURCE")
        .expect("Set CNU_BP_SOURCE to pinned upstream checkout; see README.md");
    let out = std::process::Command::new("git")
        .args(["-C", &source, "rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "b247e1ec8ed62b9abf123dc83189d253e17d488d"
    );
    let status = std::process::Command::new("git")
        .args(["-C", &source, "status", "--porcelain"])
        .output()
        .unwrap();
    assert!(
        status.status.success() && status.stdout.is_empty(),
        "Upstream must be clean"
    );
    // Apply only the reviewed local scratch cleanup delta to an OUT_DIR copy.
    println!("cargo:rerun-if-changed={source}/src");
    println!("cargo:rerun-if-changed={source}/include");
    let patched = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("upstream");
    for part in ["src", "include"] {
        copy_tree(
            &std::path::Path::new(&source).join(part),
            &patched.join(part),
        );
    }
    let header = patched.join("src/modules/bulletproofs/rangeproof_impl.h");
    let mut content = std::fs::read_to_string(&header)
        .unwrap()
        .replace("\r\n", "\n");
    for condition in [
        "!secp256k1_bulletproof_deserialize_point(&sge, &proof[i][64], 1, 4))",
        "!secp256k1_bulletproof_deserialize_point(&ecmult_data[i].t2, &proof[i][64], 3, 4))",
    ] {
        let before = format!("{condition} {{\n            return 0;");
        let after=format!("{condition} {{\n            secp256k1_scratch_deallocate_frame(scratch);\n            return 0;");
        assert_eq!(
            content.matches(&before).count(),
            1,
            "Pinned cleanup patch context changed"
        );
        content = content.replace(&before, &after);
    }
    std::fs::write(header, content).unwrap();
    let source = patched.to_str().unwrap();
    let mut c = cc::Build::new();
    c.include(source)
        .include(format!("{source}/include"))
        .include(format!("{source}/src"));
    for d in [
        "VERIFY",
        "USE_NUM_NONE",
        "USE_FIELD_INV_BUILTIN",
        "USE_SCALAR_INV_BUILTIN",
        "USE_FIELD_10X26",
        "USE_SCALAR_8X32",
        "USE_ENDOMORPHISM",
        "ENABLE_MODULE_GENERATOR",
        "ENABLE_MODULE_COMMITMENT",
        "ENABLE_MODULE_RANGEPROOF",
        "ENABLE_MODULE_BULLETPROOF",
    ] {
        c.define(d, "1");
    }
    c.warnings(false)
        .file(format!("{source}/src/secp256k1.c"))
        .file("adapter.c")
        .compile("cnu_historical_bp");
    println!("cargo:rerun-if-env-changed=CNU_BP_SOURCE");
    println!("cargo:rerun-if-changed=adapter.c");
}
