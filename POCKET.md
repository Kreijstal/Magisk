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
- Collect script modules and their properties without attempting kernel mounts.
  Modules requiring system overlays, SELinux rules, Riru or Zygisk are skipped.
- Write guest diagnostics to `/logs/magisk.log`.

This does not implement kernel modules, Magic Mount, mount namespaces, SELinux
policy patching, Zygisk, boot-image installation, or hardware-backed trust.
Installing Specter and its complete dependency stack remains unverified.

## Building and integration

The matching runtime, pinned build helper, init configuration and device tests
are maintained in [Kreijstal/aosp-pocket](https://github.com/Kreijstal/aosp-pocket),
branch `pocket-magisk`. From that project:

```sh
make -f runtime/Makefile magisk-pocket
```

The helper uses ONDK r29.5, this pinned Magisk revision and its submodules, and
versioned Termux ARM64 packages. It builds only Magisk's native core and the
necessary Pocket helpers. Private signing identities and administration tokens
are not included.

Device validation on an unrooted Samsung SM-G780G has passed the actual daemon
version handshake, database access policy, and MagiskSU returning logical guest
UID 0. The official module ZIP installer, module activation across standalone
AOSP init restarts, script ordering, and resetprop create/update/delete also
passed. Pocket enables owner writes on its private guest property files because
logical root lacks the host kernel's DAC override capability. Complete Android
UI boot and manager UI integration remain under validation. See the matching
Pocket repository for current results.

The original Magisk GPL-3.0 license and copyright notices remain applicable.
