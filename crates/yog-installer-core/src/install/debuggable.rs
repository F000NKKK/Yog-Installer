//! A JDK made debuggable on macOS: its `java` re-signed (ad hoc) with
//! `get-task-allow`, which macOS wants of a process before any debugger may
//! attach to it — Yog-IDLE's native debugger, to the game. A release JDK's
//! own signature hasn't that right; the new one keeps the rights it has
//! (the JIT's, loading the JDK's own libraries).

use super::{Context, run};
use std::path::Path;

const CODESIGN: &str = "/usr/bin/codesign";
/// Written next to the downloads for `codesign` to read.
const ENTITLEMENTS_FILE: &str = "jdk-debuggable.entitlements";
const ENTITLEMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>com.apple.security.get-task-allow</key>
	<true/>
	<key>com.apple.security.cs.allow-jit</key>
	<true/>
	<key>com.apple.security.cs.allow-unsigned-executable-memory</key>
	<true/>
	<key>com.apple.security.cs.disable-library-validation</key>
	<true/>
</dict>
</plist>
"#;

/// Re-signs the JDK at `home` (its `Contents/Home`) so the game it runs can be debugged.
pub(super) fn allow_debugging(home: &Path, id: &str, ctx: &Context) -> Result<(), String> {
    let entitlements = ctx.home.cache_dir().join(ENTITLEMENTS_FILE);
    std::fs::write(&entitlements, ENTITLEMENTS)
        .map_err(|e| format!("{}: {e}", entitlements.display()))?;
    let java = home.join("bin").join("java");
    let args = [
        "--force".to_string(),
        "--sign".to_string(),
        "-".to_string(),
        "--options".to_string(),
        "runtime".to_string(),
        "--entitlements".to_string(),
        entitlements.display().to_string(),
        java.display().to_string(),
    ];
    run::tool(Path::new(CODESIGN), &args, ctx.cancel, &ctx.output(id))
}
