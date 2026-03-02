//! Workspace automation. Run from repo root: cargo run -p xtask -- release <VERSION>

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str);
    let version = args.get(2).or_else(|| args.get(1)).map(String::as_str);

    match (command, version) {
        (Some("release"), Some(v)) if !v.is_empty() => {
            if let Err(e) = release(v) {
                let _ = io::stderr().write_fmt(format_args!("Error: {}\n", e));
                std::process::exit(1);
            }
        }
        (Some(v), _) if v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
            if let Err(e) = release(v) {
                let _ = io::stderr().write_fmt(format_args!("Error: {}\n", e));
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Usage: cargo run -p xtask -- release <VERSION>");
            eprintln!("Example: cargo run -p xtask -- release 0.4.0");
            std::process::exit(1);
        }
    }
}

fn release(new_version: &str) -> Result<(), String> {
    let root = env::current_dir().expect("current dir");
    let root = root.as_path();

    // 1. Ensure git working tree is clean
    check_git_clean(root)?;

    // 2. Read current version from gochu-core
    let core_toml = root.join("gochu-core/Cargo.toml");
    let current = read_version(&core_toml)?;
    eprintln!("Current version: {}", current);
    eprintln!("New version:     {}", new_version);

    // 3. Bump version in all three crates
    for name in ["gochu-core", "gochu-wasm", "gochu-ibus"] {
        let path = root.join(name).join("Cargo.toml");
        bump_version(&path, &current, new_version)?;
    }

    // 4. Run tests
    eprintln!("Running tests...");
    run_cmd(root, "cargo", &["test", "--workspace"])?;

    // 5. Rebuild web demo (wasm-pack + version.js)
    eprintln!("Building web demo (wasm-pack)...");
    run_wasm_build(root)?;
    write_version_js(root)?;

    // 6. Git add, commit, tag, push
    let tag = format!("v{}", new_version);
    git_add(root, &["gochu-core/Cargo.toml", "gochu-wasm/Cargo.toml", "gochu-ibus/Cargo.toml", "docs/pkg", "docs/version.js", "Cargo.lock"])?;
    git_commit(root, &format!("Bump version to {} and rebuild web demo", new_version))?;
    git_tag(root, &tag)?;
    git_push(root, "main")?;
    git_push(root, &tag)?;

    eprintln!("Release {} created and pushed.", tag);
    Ok(())
}

fn check_git_clean(root: &Path) -> Result<(), String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("git status: {}", e))?;
    if !out.stdout.is_empty() {
        return Err("git working tree is not clean. Commit or stash changes first.".into());
    }
    Ok(())
}

fn read_version(path: &Path) -> Result<String, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    for line in s.lines() {
        let line = line.trim();
        if line.starts_with("version = \"") {
            let start = line.find('"').unwrap() + 1;
            let rest = &line[start..];
            let end = rest.find('"').ok_or("version value has no closing quote")?;
            return Ok(rest[..end].to_string());
        }
    }
    Err("no version = \"...\" found in Cargo.toml".into())
}

fn bump_version(path: &Path, current: &str, new_version: &str) -> Result<(), String> {
    let s = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let needle = format!("version = \"{}\"", current);
    let replacement = format!("version = \"{}\"", new_version);
    if !s.contains(&needle) {
        return Err(format!("{}: did not find {}", path.display(), needle));
    }
    let new_s = s.replace(&needle, &replacement);
    fs::write(path, new_s).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}

fn run_cmd(root: &Path, bin: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(bin)
        .current_dir(root)
        .args(args)
        .status()
        .map_err(|e| format!("{}: {}", bin, e))?;
    if !status.success() {
        return Err(format!("{} {} failed", bin, args.join(" ")));
    }
    Ok(())
}

fn run_wasm_build(root: &Path) -> Result<(), String> {
    let out = Command::new("wasm-pack")
        .current_dir(root)
        .args(["build", "gochu-wasm", "--target", "web", "--out-dir", "../docs/pkg"])
        .output()
        .map_err(|e| format!("wasm-pack: {}. Is wasm-pack installed? (cargo install wasm-pack)", e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "wasm-pack build failed. Is wasm-pack installed? (cargo install wasm-pack)\nstderr: {}",
            stderr.trim()
        ));
    }
    let gitignore = root.join("docs/pkg/.gitignore");
    if gitignore.exists() {
        fs::remove_file(&gitignore).map_err(|e| format!("remove .gitignore: {}", e))?;
    }
    Ok(())
}

fn write_version_js(root: &Path) -> Result<(), String> {
    let commit = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None })
        .unwrap_or_else(|| "unknown".into());
    let date = Command::new("git")
        .current_dir(root)
        .args(["log", "-1", "--format=%cI"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None })
        .unwrap_or_else(|| "unknown".into());
    let content = format!(
        "window.GOCHU_VERSION = {{ commit: \"{}\", date: \"{}\" }};\n",
        commit, date
    );
    let path = root.join("docs/version.js");
    fs::write(&path, content).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}

fn git_add(root: &Path, paths: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(root).arg("add");
    for p in paths {
        cmd.arg(p);
    }
    let status = cmd.status().map_err(|e| format!("git add: {}", e))?;
    if !status.success() {
        return Err("git add failed".into());
    }
    Ok(())
}

fn git_commit(root: &Path, message: &str) -> Result<(), String> {
    let status = Command::new("git")
        .current_dir(root)
        .args(["commit", "-m", message])
        .status()
        .map_err(|e| format!("git commit: {}", e))?;
    if !status.success() {
        return Err("git commit failed".into());
    }
    Ok(())
}

fn git_tag(root: &Path, tag: &str) -> Result<(), String> {
    let status = Command::new("git")
        .current_dir(root)
        .args(["tag", tag])
        .status()
        .map_err(|e| format!("git tag: {}", e))?;
    if !status.success() {
        return Err("git tag failed".into());
    }
    Ok(())
}

fn git_push(root: &Path, refspec: &str) -> Result<(), String> {
    let status = Command::new("git")
        .current_dir(root)
        .args(["push", "origin", refspec])
        .status()
        .map_err(|e| format!("git push: {}", e))?;
    if !status.success() {
        return Err(format!("git push origin {} failed", refspec));
    }
    Ok(())
}
