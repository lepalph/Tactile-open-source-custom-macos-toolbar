//! `cfg(ci)` quand la variable `CI` est posée : les tests qui touchent le matériel (écran, son, batterie)
//! y sont marqués `ignored` par `#[cfg_attr(ci, ignore = …)]`.
fn main() {
    println!("cargo::rerun-if-env-changed=CI");
    println!("cargo::rustc-check-cfg=cfg(ci)");
    if std::env::var_os("CI").is_some() {
        println!("cargo::rustc-cfg=ci");
    }
}
