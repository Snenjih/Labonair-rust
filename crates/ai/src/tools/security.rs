//! Path-safety and shell-command guards for AI tool calls (T11-004).
//!
//! Pure-Rust port of `reference-src/src/modules/ai/lib/security.ts`.
//!
//! Goals:
//! * Block reads of files that almost always contain secrets (`.env*`, `*.pem`,
//!   `id_rsa*`, `.aws/credentials`, `.ssh/`, `.git/`, …).
//! * Block writes/exec into the same set, plus system directories where
//!   automated mutation is dangerous.
//!
//! This is a *defense layer*, not a sandbox. The user-confirmation UI for
//! write/exec is the real safety net; these checks ensure read tools (which
//! auto-approve) can never silently exfiltrate obvious secrets, and that a
//! single bad approval can't blow up the system.

/// Result of a safety check. `Err` carries a human-readable reason.
pub type SafetyResult = Result<(), String>;

const SECRET_BASENAME_PATTERNS: &[fn(&str) -> bool] = &[
    // .env, .env.local, .env.production, …
    |b| b == ".env" || b.starts_with(".env."),
    |b| b.ends_with(".pem"),
    |b| b.ends_with(".key"),
    |b| b.ends_with(".p12"),
    |b| b.ends_with(".pfx"),
    |b| b.ends_with(".asc"),
    |b| b.ends_with(".gpg"),
    |b| b.ends_with(".jks"),
    |b| b.ends_with(".keystore"),
    // id_rsa, id_dsa, id_ecdsa, id_ed25519 (+ .pub)
    |b| {
        let b = b.strip_suffix(".pub").unwrap_or(b);
        matches!(b, "id_rsa" | "id_dsa" | "id_ecdsa" | "id_ed25519")
    },
    |b| b == "known_hosts",
    |b| b == "authorized_keys",
    |b| b == "htpasswd",
    |b| b == ".netrc",
    |b| b == "credentials",
    |b| b == ".pgpass",
    |b| b == ".npmrc",
    |b| b == ".pypirc",
    |b| {
        matches!(
            b,
            "secret.json"
                | "secrets.json"
                | "secret.yaml"
                | "secrets.yaml"
                | "secret.yml"
                | "secrets.yml"
                | "secret.toml"
                | "secrets.toml"
        )
    },
    |b| b.starts_with("service_account") && b.ends_with(".json"),
];

const SECRET_PATH_SEGMENTS: &[&str] = &[
    "/.ssh/",
    "/.gnupg/",
    "/.aws/",
    "/.azure/",
    "/.kube/",
    "/.docker/",
    "/.config/gh/",
    "/.config/git/",
    "/.config/gcloud/",
    "/.git/",
    "/var/root/",
    "/private/var/root/",
    "/appdata/roaming/",
];

const FORBIDDEN_PREFIXES: &[&str] = &[
    "/etc/",
    "/var/db/",
    "/system/",
    "/library/keychains/",
    "/private/etc/",
    "/private/var/db/",
    "/proc/",
    "/sys/",
    "/var/root/",
    "/private/var/root/",
];

fn basename(p: &str) -> &str {
    match p.rfind(['/', '\\']) {
        Some(i) => &p[i + 1..],
        None => p,
    }
}

/// Normalize a path for matching: backslash → slash, strip UNC prefix, drive
/// letter, NTFS alternate-data-stream suffix, trailing dots/spaces per segment,
/// lowercase.
fn normalize(p: &str) -> String {
    let mut s = p.replace('\\', "/");
    if let Some(rest) = s.strip_prefix("/?/") {
        s = format!("/{rest}");
    }
    // Strip a leading Windows drive letter ("C:").
    if s.len() >= 2 {
        let b = s.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' {
            s = s[2..].to_string();
        }
    }
    // Strip NTFS alternate data streams (":stream") from each segment.
    s = s
        .split('/')
        .map(|seg| seg.split(':').next().unwrap_or(seg))
        .collect::<Vec<_>>()
        .join("/");
    // Strip trailing dots/spaces per segment (Windows discards them).
    s = s
        .split('/')
        .map(|seg| seg.trim_end_matches(['.', ' ']))
        .collect::<Vec<_>>()
        .join("/");
    s.to_lowercase()
}

/// Refuse obvious secret files / protected directories on a *read* path.
pub fn check_readable(path: &str) -> SafetyResult {
    let norm = normalize(path);
    let base = basename(&norm);

    for pat in SECRET_BASENAME_PATTERNS {
        if pat(base) {
            return Err(format!(
                "Refused: \"{base}\" matches a sensitive-file pattern."
            ));
        }
    }
    for seg in SECRET_PATH_SEGMENTS {
        if norm.contains(seg) {
            return Err(format!(
                "Refused: path is inside a protected directory ({}).",
                seg.replace('/', "")
            ));
        }
    }
    Ok(())
}

/// Refuse the read restrictions plus system-directory writes on a *write* path.
pub fn check_writable(path: &str) -> SafetyResult {
    check_readable(path)?;
    let norm = normalize(path);
    for prefix in FORBIDDEN_PREFIXES {
        if norm.starts_with(prefix) {
            return Err(format!(
                "Refused: writes under \"{prefix}\" are not allowed."
            ));
        }
    }
    Ok(())
}

/// Path-safety check that also resolves symlinks (via `std::fs::canonicalize`)
/// and re-checks the resolved target. If the path doesn't exist yet, only the
/// literal check applies.
pub fn check_readable_resolved(path: &str) -> SafetyResult {
    check_readable(path)?;
    if let Ok(real) = std::fs::canonicalize(path) {
        check_readable(&real.to_string_lossy())?;
    }
    Ok(())
}

/// Symlink-aware counterpart to [`check_writable`].
pub fn check_writable_resolved(path: &str) -> SafetyResult {
    check_writable(path)?;
    if let Ok(real) = std::fs::canonicalize(path) {
        check_writable(&real.to_string_lossy())?;
    }
    Ok(())
}

/// A human-readable warning label when a command matches a destructive pattern,
/// or `None` when it looks safe. Does **not** block — surfaced in the approval
/// UI so the user knows what they're about to allow.
pub fn check_destructive_command(cmd: &str) -> Option<&'static str> {
    let lc = cmd.to_lowercase();
    if rm_rf_regex(&lc, false) {
        return Some("Recursive force delete (rm -rf)");
    }
    if lc.contains("drop table") || lc.contains("drop database") || lc.contains("drop schema") {
        return Some("SQL DROP statement");
    }
    if lc.contains("truncate table") {
        return Some("SQL TRUNCATE");
    }
    if lc.contains("git reset --hard") {
        return Some("git reset --hard");
    }
    if lc.contains("git push") && lc.contains("--force") {
        return Some("git force push");
    }
    if contains_chmod_777(&lc) {
        return Some("chmod 777");
    }
    None
}

fn contains_chmod_777(lc: &str) -> bool {
    // chmod [ -R ] 777
    let mut it = lc.split_whitespace().peekable();
    while let Some(tok) = it.next() {
        if tok == "chmod" {
            let mut n = it.clone();
            if let Some(next) = n.next() {
                if next == "-r" {
                    if n.next() == Some("777") {
                        return true;
                    }
                } else if next == "777" {
                    return true;
                }
            }
        }
    }
    false
}

/// `rm -rf <path>` where `path` starts a token — used both by
/// [`check_destructive_command`] (any path) and [`check_shell_command`]
/// (filesystem root only, via `root_only`).
fn rm_rf_regex(lc: &str, _root_only: bool) -> bool {
    let toks: Vec<&str> = lc.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        if *t != "rm" {
            continue;
        }
        // scan following flag tokens for a combined r+f
        let mut has_r = false;
        let mut has_f = false;
        for t2 in &toks[i + 1..] {
            if let Some(flags) = t2.strip_prefix('-') {
                if let Some(long) = flags.strip_prefix('-') {
                    if long == "recursive" {
                        has_r = true;
                    }
                    if long == "force" {
                        has_f = true;
                    }
                } else {
                    if flags.contains('r') {
                        has_r = true;
                    }
                    if flags.contains('f') {
                        has_f = true;
                    }
                }
            } else {
                break;
            }
        }
        if has_r && has_f {
            return true;
        }
    }
    false
}

/// Lightweight heuristic that *blocks* obviously catastrophic shell commands
/// even after the user approved them (`rm -rf /`, `dd of=/dev/…`, `mkfs`, …) and
/// refuses any command that references a secret file/dir the read tools refuse.
///
/// Known limitation: string-matching, not a shell parser — command
/// substitution can evade it. The approval UI is the real gate.
pub fn check_shell_command(cmd: &str) -> SafetyResult {
    let trimmed = cmd.trim();
    let c = trimmed
        .strip_prefix("sudo ")
        .or_else(|| trimmed.strip_prefix("doas "))
        .unwrap_or(trimmed);
    let lc = c.to_lowercase();

    if rm_targets_root(&lc) {
        return Err(
            "Refused: command attempts to recursively delete the filesystem root.".to_string(),
        );
    }
    if lc.contains("--no-preserve-root") {
        return Err("Refused: --no-preserve-root is not allowed.".to_string());
    }
    if dd_to_block_device(&lc) {
        return Err("Refused: dd to a block device is not allowed.".to_string());
    }
    if formats_disk(&lc) {
        return Err("Refused: disk-formatting commands are not allowed.".to_string());
    }

    for raw in c.split_whitespace() {
        let token = raw.trim_matches(['\'', '"']);
        if token.is_empty() {
            continue;
        }
        let norm = normalize(token);
        let base = basename(&norm);
        for pat in SECRET_BASENAME_PATTERNS {
            if pat(base) {
                return Err(format!(
                    "Refused: command references a sensitive-file pattern (\"{base}\")."
                ));
            }
        }
        for seg in SECRET_PATH_SEGMENTS {
            if norm.contains(seg) {
                return Err(format!(
                    "Refused: command references a protected directory ({}).",
                    seg.replace('/', "")
                ));
            }
        }
    }
    Ok(())
}

fn rm_targets_root(lc: &str) -> bool {
    if !rm_rf_regex(lc, true) {
        return false;
    }
    // The token after the flags is a bare "/" (optionally quoted, optionally
    // followed by a command separator).
    let toks: Vec<&str> = lc.split_whitespace().collect();
    for t in toks {
        let t = t.trim_matches(['\'', '"']);
        if t == "/" || t == "/;" || t == "/&" || t == "/|" {
            return true;
        }
    }
    false
}

fn dd_to_block_device(lc: &str) -> bool {
    if !lc.split_whitespace().any(|t| t == "dd") {
        return false;
    }
    lc.split_whitespace().any(|t| {
        let of = t.strip_prefix("of=").unwrap_or("");
        of.starts_with("/dev/disk")
            || of.starts_with("/dev/sd")
            || of.starts_with("/dev/nvme")
            || of.starts_with("/dev/hd")
    })
}

fn formats_disk(lc: &str) -> bool {
    for t in lc.split_whitespace() {
        if t == "mkfs" || t.starts_with("mkfs.") || t == "fdisk" || t == "parted" {
            return true;
        }
    }
    lc.contains("diskutil erase")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_secret_basenames() {
        for p in [
            "/home/user/.env",
            "/project/.env.local",
            "/project/.env.production",
            "/home/user/.ssh/id_rsa",
            "/home/user/.ssh/id_ed25519.pub",
            "/home/user/cert.pem",
            "/home/user/private.key",
            "/home/user/keystore.p12",
            "/home/user/android.keystore",
            "/home/user/.netrc",
            "/home/user/.pgpass",
            "/home/user/secrets.json",
            "/home/user/secrets.toml",
            "/home/user/service_account_prod.json",
        ] {
            assert!(check_readable(p).is_err(), "should block {p}");
        }
    }

    #[test]
    fn blocks_protected_segments() {
        for p in [
            "/home/user/.ssh/config",
            "/home/user/.aws/credentials",
            "/home/user/.kube/config",
            "/home/user/.config/gh/hosts.yml",
            "/home/user/project/.git/config",
        ] {
            assert!(check_readable(p).is_err(), "should block {p}");
        }
    }

    #[test]
    fn case_insensitive_and_windows_paths() {
        assert!(check_readable("/project/.ENV").is_err());
        assert!(check_readable("/home/user/.ssh/ID_RSA").is_err());
        assert!(check_readable("C:\\Users\\foo\\.env").is_err());
        assert!(check_readable("C:\\Users\\foo\\.ssh\\id_rsa").is_err());
        assert!(check_readable("//?/.env").is_err());
    }

    #[test]
    fn allows_safe_paths() {
        for p in [
            "src/main.rs",
            "/home/user/project/Cargo.toml",
            "/home/user/notes.txt",
            "/tmp/output.log",
            "/home/user/.bashrc",
            "/project/file.txt:metadata",
        ] {
            assert!(check_readable(p).is_ok(), "should allow {p}");
        }
    }

    #[test]
    fn writable_inherits_and_adds_system_dirs() {
        assert!(check_writable("/project/.env").is_err());
        assert!(check_writable("/home/user/.ssh/id_rsa").is_err());
        for p in [
            "/etc/hosts",
            "/var/db/x",
            "/proc/self/mem",
            "/sys/kernel/debug",
            "/private/etc/hosts",
            "/System/Library/foo",
            "/Library/Keychains/System.keychain",
        ] {
            assert!(check_writable(p).is_err(), "should block write {p}");
        }
        for p in [
            "/home/user/myfile.txt",
            "/tmp/out.json",
            "/home/user/p/src/main.rs",
        ] {
            assert!(check_writable(p).is_ok(), "should allow write {p}");
        }
    }

    #[test]
    fn destructive_command_labels() {
        assert_eq!(
            check_destructive_command("rm -rf /tmp/old_build"),
            Some("Recursive force delete (rm -rf)")
        );
        assert_eq!(
            check_destructive_command("DROP TABLE users"),
            Some("SQL DROP statement")
        );
        assert_eq!(
            check_destructive_command("drop database mydb"),
            Some("SQL DROP statement")
        );
        assert_eq!(
            check_destructive_command("TRUNCATE TABLE sessions"),
            Some("SQL TRUNCATE")
        );
        assert_eq!(
            check_destructive_command("git reset --hard HEAD~1"),
            Some("git reset --hard")
        );
        assert_eq!(
            check_destructive_command("git push origin main --force"),
            Some("git force push")
        );
        assert_eq!(
            check_destructive_command("chmod 777 /var/www"),
            Some("chmod 777")
        );
        assert_eq!(
            check_destructive_command("chmod -R 777 /var/www"),
            Some("chmod 777")
        );
        for safe in [
            "ls -la",
            "git status",
            "npm install",
            "rm file.txt",
            "git push origin main",
            "chmod 755 script.sh",
        ] {
            assert_eq!(check_destructive_command(safe), None, "safe: {safe}");
        }
    }

    #[test]
    fn shell_command_blocks_catastrophic() {
        for bad in [
            "rm -rf /",
            "rm -rf \"/\"",
            "rm -fr /",
            "rm -rf --no-preserve-root /",
            "dd if=/dev/zero of=/dev/sda",
            "dd if=/dev/zero of=/dev/disk0",
            "dd if=/dev/urandom of=/dev/nvme0n1",
            "mkfs.ext4 /dev/sdb1",
            "mkfs /dev/sdb",
            "fdisk /dev/sda",
            "diskutil eraseDisk APFS MyDisk /dev/disk2",
        ] {
            assert!(check_shell_command(bad).is_err(), "should block: {bad}");
        }
    }

    #[test]
    fn shell_command_allows_safe() {
        for ok in [
            "rm -rf /tmp/build",
            "rm -rf ./node_modules",
            "git status",
            "ls -la /",
            "dd if=/dev/zero of=./disk.img bs=1M count=10",
            "echo 'hello'",
            "npm run build",
        ] {
            assert!(check_shell_command(ok).is_ok(), "should allow: {ok}");
        }
    }

    #[test]
    fn shell_command_blocks_secret_refs() {
        assert!(check_shell_command("cat ~/.ssh/id_rsa").is_err());
        assert!(check_shell_command("less .env").is_err());
        assert!(check_shell_command("cp .aws/credentials /tmp").is_err());
        assert!(check_shell_command("cat \".env\"").is_err());
        assert!(check_shell_command("sudo rm -rf /").is_err());
    }
}
