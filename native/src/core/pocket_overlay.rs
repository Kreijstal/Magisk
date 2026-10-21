// Pocket's userspace equivalent of module mounts. The original image is never
// replaced: publish a merged private tree before zygote and resolve paths there.
use std::{fs, io, path::Path};
use std::os::unix::fs::{symlink, PermissionsExt};

fn remove(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn copy_tree(source: &Path, target: &Path, baseline: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(source)?;
    if meta.is_dir() {
        if !fs::symlink_metadata(target).is_ok_and(|m| m.is_dir()) { remove(target)?; }
        fs::create_dir_all(target)?;
        if !baseline && source.join(".replace").exists() {
            remove(target)?;
            fs::create_dir_all(target)?;
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            if !baseline && entry.file_name() == ".replace" { continue; }
            copy_tree(&entry.path(), &target.join(entry.file_name()), baseline)?;
        }
        fs::set_permissions(target, fs::Permissions::from_mode(meta.permissions().mode()))?;
    } else if meta.file_type().is_symlink() {
        remove(target)?;
        symlink(fs::read_link(source)?, target)?;
    } else if meta.is_file() {
        // Unlink first: replacing a hard-linked baseline must not modify it.
        remove(target)?;
        if baseline {
            // Android host policy can forbid hard links even between files
            // owned by this app. Fall back to a private copy in that case.
            if fs::hard_link(source, target).is_err() { fs::copy(source, target)?; }
        }
        else { fs::copy(source, target)?; }
    } else {
        return Err(io::Error::new(io::ErrorKind::Unsupported,
                                 "Pocket module views do not support special files"));
    }
    Ok(())
}

pub fn build(root: &Path, modules: &[&Path], zygisk: bool) -> io::Result<()> {
    let view = root.join(".pocket-module-view");
    remove(&view)?;
    if modules.is_empty() && !zygisk { return Ok(()); }
    fs::create_dir_all(&view)?;
    for partition in ["system", "vendor"] {
        if root.join(partition).is_dir() {
            copy_tree(&root.join(partition), &view.join(partition), true)?;
        }
    }
    for module in modules {
        let system = module.join("system");
        if !system.is_dir() || module.join("skip_mount").exists() { continue; }
        if system.join(".replace").exists() {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "replacing all /system is unsupported"));
        }
        for entry in fs::read_dir(&system)? {
            let entry = entry?;
            let name = entry.file_name();
            let target = if name == "vendor" && view.join("vendor").is_dir() {
                view.join("vendor")
            } else { view.join("system").join(name) };
            copy_tree(&entry.path(), &target, false)?;
        }
    }
    if zygisk {
        let target = view.join("system/lib64/libpocket_zygisk.so");
        // A user-installed provider module can supply the cooperative ABI.
        // The built-in Zygisk provider is only the default fallback.
        if !target.exists() {
            fs::copy(root.join("sbin/magisk"), target)?;
        }
    }
    // The runtime checks this marker; an incomplete tree never becomes active.
    fs::write(view.join("ready"), b"1\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merging_replacement_symlinks_and_removal_preserve_baseline() {
        let root = std::env::temp_dir().join(format!("pocket-overlay-{}", std::process::id()));
        remove(&root).unwrap();
        fs::create_dir_all(root.join("system/etc/replace")).unwrap();
        fs::create_dir_all(root.join("vendor/etc")).unwrap();
        fs::write(root.join("system/etc/value"), b"base").unwrap();
        fs::write(root.join("system/etc/replace/old"), b"old").unwrap();
        let module = root.join("module");
        fs::create_dir_all(module.join("system/etc/replace")).unwrap();
        fs::create_dir_all(module.join("system/vendor/etc")).unwrap();
        fs::write(module.join("system/etc/value"), b"module").unwrap();
        fs::write(module.join("system/etc/replace/.replace"), b"").unwrap();
        fs::write(module.join("system/vendor/etc/new"), b"vendor").unwrap();
        symlink("value", module.join("system/etc/link")).unwrap();
        build(&root, &[&module], false).unwrap();
        let view = root.join(".pocket-module-view");
        assert_eq!(fs::read(view.join("system/etc/link")).unwrap(), b"module");
        assert!(!view.join("system/etc/replace/old").exists());
        assert_eq!(fs::read(view.join("vendor/etc/new")).unwrap(), b"vendor");
        assert_eq!(fs::read(root.join("system/etc/value")).unwrap(), b"base");
        build(&root, &[], false).unwrap();
        assert!(!view.exists());
        assert_eq!(fs::read(root.join("system/etc/value")).unwrap(), b"base");
        remove(&root).unwrap();
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;
    #[test]
    fn user_provider_overrides_default_and_removal_restores_default() {
        let root = std::env::temp_dir().join(format!("pocket-provider-{}", std::process::id()));
        remove(&root).unwrap();
        fs::create_dir_all(root.join("system/lib64")).unwrap();
        fs::create_dir_all(root.join("sbin")).unwrap();
        fs::write(root.join("sbin/magisk"), b"default provider").unwrap();
        let module = root.join("provider-module");
        fs::create_dir_all(module.join("system/lib64")).unwrap();
        fs::write(module.join("system/lib64/libpocket_zygisk.so"), b"user provider").unwrap();
        build(&root, &[&module], true).unwrap();
        let selected = root.join(".pocket-module-view/system/lib64/libpocket_zygisk.so");
        assert_eq!(fs::read(&selected).unwrap(), b"user provider");
        assert!(!root.join("system/lib64/libpocket_zygisk.so").exists());
        build(&root, &[], true).unwrap();
        assert_eq!(fs::read(&selected).unwrap(), b"default provider");
        remove(&root).unwrap();
    }
}
