use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=FILELOCK_FORCE_BACKPORT");
    let version = rustc_version();
    let supports_check_cfg =
        matches!(version, Some((major, minor)) if major > 1 || (major == 1 && minor >= 80));
    if supports_check_cfg {
        println!("cargo:rustc-check-cfg=cfg(filelock_has_std_lock)");
        println!("cargo:rustc-check-cfg=cfg(filelock_std_lock)");
    }

    let has_std_lock =
        matches!(version, Some((major, minor)) if major > 1 || (major == 1 && minor >= 89));
    if has_std_lock {
        println!("cargo:rustc-cfg=filelock_has_std_lock");
    }

    let force_backport =
        matches!(env::var_os("FILELOCK_FORCE_BACKPORT"), Some(value) if value == "1");
    if has_std_lock && !force_backport {
        println!("cargo:rustc-cfg=filelock_std_lock");
    }
}

fn rustc_version() -> Option<(u32, u32)> {
    let rustc = env::var_os("RUSTC")?;
    let output = Command::new(rustc).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let version = std::str::from_utf8(&output.stdout)
        .ok()?
        .split_whitespace()
        .nth(1)?;
    let mut components = version.split('.');
    let major = components.next()?.parse::<u32>().ok()?;
    let minor = components.next()?.parse::<u32>().ok()?;
    Some((major, minor))
}
