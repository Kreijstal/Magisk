# Magisk for AOSP Pocket

Experimental userspace integration based on Magisk 30.7
(`e8a58776f1d7bdf852072ad0baa6eceb9a1e4aac`).

This branch runs the actual Magisk daemon, CLI, MagiskSU and module scripts in
AOSP Pocket's Android 9 guest on an unrooted Android host. It requires Pocket's
native path/UID compatibility runtime. It does not root the host or patch its
kernel or boot image.

The host app UID and SELinux domain remain the real security boundary. Logical
guest UIDs are compatibility bookkeeping. This is not a separate kernel or a
secure boundary against guest code issuing raw syscalls.

## Changes

- Validate the kernel socket owner, then restore the peer's logical guest UID
  from Pocket's exec environment for Magisk permission handling.
- Keep the daemon in the foreground when supervised by Pocket's AOSP init.
- Run post-fs-data and service scripts using a dynamic Termux BusyBox and an
  explicit applet PATH; bypass the stock standalone-ash re-execution helper.
- Use the guest's prepared filesystem instead of probing/mounting host ROM
  partitions. Keep host SELinux labels and cgroups unchanged.
- Publish a private merged system/vendor file view before zygote. Module
  replacements, additions, skip_mount and .replace directories do not modify
  the base image or require host kernel mounts.
- Install the upstream Zygisk JNI wrappers through Pocket's controlled Android
  9 registration, retaining module callbacks and the public JNI/PLT APIs.
  Modules requiring host SELinux rules or Riru remain unsupported.
- Write guest diagnostics to `/logs/magisk.log`.

This does not implement kernel mounts, kernel modules, mount namespaces, SELinux
policy patching, device-node whiteouts, boot-image installation, or hardware-backed trust.
Installing Specter and its complete dependency stack remains unverified.

## Building and integration

The matching runtime, pinned build helper, init configuration and device tests
are maintained in [Kreijstal/aosp-pocket](https://github.com/Kreijstal/aosp-pocket),
branch `pocket-magisk`. From that project:

```sh
make -f runtime/Makefile magisk-pocket
```

Magisk is optional and is not bundled in the Pocket APK. That APK provides the
VM compatibility adapters only; the target above builds a separate optional
runtime archive for explicit installation or testing. The support-only APK
target does not fetch or compile Magisk. A Manager app is a separate install.

The optional helper uses ONDK r29.5, this pinned Magisk revision and its submodules, and
versioned Termux ARM64 packages. It builds only Magisk's native core and the
necessary Pocket helpers. Private signing identities and administration tokens
are not included.

Device validation on an unrooted Samsung SM-G780G has passed the actual daemon
version handshake, database access policy, and MagiskSU returning logical guest
UID 0. The official module ZIP installer, module activation across standalone
AOSP init restarts, script ordering, and resetprop create/update/delete also
passed. Pocket enables owner writes on its private guest property files because
logical root lacks the host kernel's DAC override capability. Complete Android
UI boot and manager UI integration remain under validation. Actual guest tests
also passed Zygisk onLoad/pre/post app callbacks, JNI interception, module file
overlays, rollback after disabling a module, and init without Magisk installed.
See the matching
Pocket repository for current results.

The original Magisk GPL-3.0 license and copyright notices remain applicable.
