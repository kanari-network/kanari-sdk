// Port of tools/vscode-move-kanari/src/deps/cacheRepair.ts.
// Repairs ~/.move git dependency caches before symbolication so
// move-analyzer does not start with a stale index.lock / dirty index
// that makes "Failed to resolve dependencies" suppress all diagnostics.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the move dependency home (respects MOVE_HOME).
pub fn move_home() -> PathBuf {
    if let Ok(env) = std::env::var("MOVE_HOME")
        && !env.trim().is_empty()
    {
        return PathBuf::from(env);
    }
    if let Some(home) = dirs_home() {
        return home.join(".move");
    }
    PathBuf::from(".move")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

/// Every clone under MOVE_HOME that is a git work tree.
pub fn dependency_caches() -> Vec<PathBuf> {
    let home = move_home();
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&home) else {
        return out;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if dir.is_dir() && dir.join(".git").exists() {
            out.push(dir);
        }
    }
    out
}

struct GitResult {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn git(cwd: &Path, args: &[&str]) -> GitResult {
    let output = Command::new("git").arg("-C").arg(cwd).args(args).output();
    match output {
        Ok(o) => GitResult {
            ok: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        },
        Err(e) => GitResult {
            ok: false,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

/// Repair all caches. Returns (num_repaired, num_unrepairable).
pub fn repair_dependency_caches() -> (usize, usize) {
    if Command::new("git").arg("--version").output().is_err() {
        return (0, 0);
    }
    let caches = dependency_caches();
    if caches.is_empty() {
        return (0, 0);
    }
    let mut repaired = 0;
    let mut unrepairable = 0;
    for cache in &caches {
        match repair_one(cache) {
            Ok(()) => repaired += 1,
            Err(reason) => {
                unrepairable += 1;
                eprintln!("kanari-move-analyzer: cache repair failed for {cache:?}: {reason}");
            }
        }
    }
    (repaired, unrepairable)
}

fn repair_one(cache: &Path) -> Result<(), String> {
    // 1. longpaths so `git reset --hard` does not fail on MAX_PATH
    let lp = git(cache, &["config", "core.longpaths", "true"]);
    if !lp.ok {
        eprintln!(
            "kanari-move-analyzer: could not set core.longpaths in {cache:?}: {}",
            lp.stderr.trim()
        );
    }

    // 2. index.lock: probe with `git status`, wait up to 5s for in-flight
    //    fetch to finish, then remove the (advisory) lock. Unrelated git
    //    processes elsewhere on the machine must not count (the old TS
    //    `tasklist` check false-positived on VS Code's own git).
    let lock = cache.join(".git").join("index.lock");
    if lock.exists() {
        let mut waited = 0;
        while lock.exists() && waited < 5000 {
            let probe = git(cache, &["status", "--porcelain"]);
            if probe.ok {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            waited += 500;
        }
        if lock.exists() {
            std::fs::remove_file(&lock).map_err(|e| format!("could not remove {lock:?}: {e}"))?;
            eprintln!("kanari-move-analyzer: removed stale {}", lock.display());
        }
    }

    // 3. dirty index -> reset to pinned revision
    let status = git(cache, &["status", "--porcelain"]);
    if !status.ok {
        return Err(status.stderr.trim().to_string());
    }
    if !status.stdout.trim().is_empty() {
        let reset = git(cache, &["reset", "--hard", "--quiet"]);
        if !reset.ok {
            return Err(reset.stderr.trim().to_string());
        }
    }

    // 4. final check
    let fin = git(cache, &["status", "--porcelain"]);
    if !fin.ok {
        return Err(fin.stderr.trim().to_string());
    }
    if !fin.stdout.trim().is_empty() {
        return Err("cache is still dirty after repair".to_string());
    }
    Ok(())
}
