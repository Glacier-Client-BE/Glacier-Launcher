use gl_modpack::{export_mrpack, ExportOptions};
use std::fs;

#[test]
fn export_mrpack_roundtrip_via_manifest_parser() {
    let tmp = std::env::temp_dir().join(format!("gl-modpack-export-test-{}", std::process::id()));
    let instance = tmp.join("instance");
    fs::create_dir_all(instance.join("mods")).unwrap();
    fs::create_dir_all(instance.join("config")).unwrap();
    fs::write(instance.join("mods/local.jar"), b"fake-jar-bytes").unwrap();
    fs::write(instance.join("config/settings.txt"), b"opt=1").unwrap();

    let dest = tmp.join("out.mrpack");
    let opts = ExportOptions {
        pack_name: "Test Pack".into(),
        pack_version: "1.0.0".into(),
        minecraft_version: "1.20.1".into(),
        loader: "fabric".into(),
        loader_version: "0.15.0".into(),
    };
    export_mrpack(&instance, &dest, &opts).unwrap();

    // Re-parse it the same way the existing import path does.
    let file = std::fs::File::open(&dest).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let plan = gl_modpack::manifest::parse_modrinth_index(&mut archive, "fallback").unwrap();
    assert_eq!(plan.name, "Test Pack");
    assert_eq!(plan.minecraft_version, "1.20.1");
    assert_eq!(plan.loader_version, "0.15.0");
    // local.jar had no provenance/url, so it went to overrides, not files[].
    assert_eq!(plan.files.len(), 0);
    assert!(archive.by_name("overrides/mods/local.jar").is_ok());
    assert!(archive.by_name("overrides/config/settings.txt").is_ok());

    fs::remove_dir_all(&tmp).ok();
}
