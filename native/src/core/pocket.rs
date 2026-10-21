// Pocket guest integration. Kernel credentials and host SELinux remain unchanged.
use std::os::unix::net::UCred;

pub fn enabled() -> bool {
    std::env::var("POCKET_DIRECT_ROOT").is_ok_and(|root| root.starts_with('/'))
}

// All guest processes share one host app UID. Restore the logical credentials
// carried by Pocket's exec boundary, after validating the kernel socket owner.
// This is guest policy bookkeeping, not an additional host security boundary.
pub fn peer_identity(cred: &mut UCred) -> bool {
    if !enabled() { return true; }
    let host_uid = unsafe { base::libc::syscall(base::libc::SYS_getuid) as u32 };
    if cred.uid != host_uid { return false; }
    let Some(pid) = cred.pid else { return false; };
    let Ok(environment) = std::fs::read(format!("/proc/{pid}/environ")) else { return false; };
    if environment.len() > 1024 * 1024 { return false; }
    for item in environment.split(|c| *c == 0) {
        if let Some(value) = item.strip_prefix(b"POCKET_DIRECT_ID=") {
            let Ok(value) = std::str::from_utf8(value) else { return false; };
            let Some((uid, gid)) = value.split_once(':') else { return false; };
            let (Ok(uid), Ok(gid)) = (uid.parse::<u32>(), gid.parse::<u32>()) else { return false; };
            cred.uid = uid; cred.gid = gid;
            return true;
        }
    }
    false
}
