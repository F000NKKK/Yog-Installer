//! What's installed through rustup and cargo: the Rust toolchain, its
//! targets, `cargo install`ed tools. rustup itself is fetched when missing.

use super::{Context, run};
use crate::platform;
use std::path::PathBuf;

/// rustup-init for a host triple.
fn rustup_init_url(host: &str) -> String {
    format!(
        "https://static.rust-lang.org/rustup/dist/{host}/{}",
        platform::exe("rustup-init")
    )
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|a| a.to_string()).collect()
}

/// rustup, installed first when the machine has none.
fn rustup(id: &str, ctx: &Context) -> Result<PathBuf, String> {
    let rustup = ctx.cargo_bin.join(platform::exe("rustup"));
    if rustup.is_file() {
        return Ok(rustup);
    }
    let host = platform::rustup_host().ok_or("Rust has no toolchain for this platform")?;
    let init = ctx.home.cache_dir().join(platform::exe("rustup-init"));
    crate::download::download(&rustup_init_url(host), &init, None, ctx.cancel, &|_, _| {})?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&init, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    // Just rustup: the toolchain comes next, as its own step.
    run::tool(
        &init,
        &args(&["-y", "--default-toolchain", "none", "--profile", "minimal"]),
        ctx.cancel,
        &ctx.output(id),
    )?;
    let _ = std::fs::remove_file(&init);
    Ok(rustup)
}

pub fn toolchain(id: &str, toolchain: &str, ctx: &Context) -> Result<PathBuf, String> {
    let rustup = rustup(id, ctx)?;
    run::tool(
        &rustup,
        &args(&["toolchain", "install", toolchain, "--profile", "minimal"]),
        ctx.cancel,
        &ctx.output(id),
    )?;
    Ok(ctx.cargo_bin.clone())
}

pub fn toolchain_uninstall(id: &str, toolchain: &str, ctx: &Context) -> Result<(), String> {
    run::tool(
        &rustup(id, ctx)?,
        &args(&["toolchain", "uninstall", toolchain]),
        ctx.cancel,
        &ctx.output(id),
    )
}

pub fn targets(
    id: &str,
    toolchain: &str,
    targets: &[String],
    ctx: &Context,
) -> Result<PathBuf, String> {
    let mut a = args(&["target", "add", "--toolchain", toolchain]);
    a.extend(targets.iter().cloned());
    run::tool(&rustup(id, ctx)?, &a, ctx.cancel, &ctx.output(id))?;
    Ok(ctx.cargo_bin.clone())
}

pub fn targets_remove(
    id: &str,
    toolchain: &str,
    targets: &[String],
    ctx: &Context,
) -> Result<(), String> {
    let mut a = args(&["target", "remove", "--toolchain", toolchain]);
    a.extend(targets.iter().cloned());
    run::tool(&rustup(id, ctx)?, &a, ctx.cancel, &ctx.output(id))
}

pub fn cargo_install(
    id: &str,
    name: &str,
    version: &str,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let cargo = ctx.cargo_bin.join(platform::exe("cargo"));
    run::tool(
        &cargo,
        &args(&["install", "--locked", &format!("{name}@{version}")]),
        ctx.cancel,
        &ctx.output(id),
    )?;
    Ok(ctx.cargo_bin.join(platform::exe(name)))
}

pub fn cargo_uninstall(id: &str, name: &str, ctx: &Context) -> Result<(), String> {
    let cargo = ctx.cargo_bin.join(platform::exe("cargo"));
    run::tool(
        &cargo,
        &args(&["uninstall", name]),
        ctx.cancel,
        &ctx.output(id),
    )
}
