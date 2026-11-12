//! ARMv7/musl compatibility for the ADM 3.10 kernel and Docker 18 seccomp policy.
//!
//! The kernel supports the legacy clock syscall, but not clock_gettime64. The old
//! Docker filter returns EPERM for the unknown time64 syscall; musl only falls
//! back on ENOSYS. Use the existing allowed ARM clock syscall and widen its result.
//! This is opt-in, does not change seccomp, and inherits the kernel's 2038 limit.
use libc::c_int;

// The prebuilt Rust standard library uses musl's time64 ABI, independently of
// the registry libc crate's optional time64 cfg. Spell out that ABI explicitly.
#[repr(C)]
pub struct Timespec64 {
    tv_sec: i64,
    tv_nsec: i32,
    padding: i32,
}

unsafe fn legacy_clock(clock: libc::clockid_t, output: *mut Timespec64) -> c_int {
    if output.is_null() {
        *libc::__errno_location() = libc::EFAULT;
        return -1;
    }
    let mut old = [0i32; 2];
    let result = libc::syscall(libc::SYS_clock_gettime, clock, old.as_mut_ptr());
    if result != 0 {
        return -1;
    }
    // Zero ABI padding as well as the visible fields before widening time32.
    std::ptr::write_bytes(output, 0, 1);
    (*output).tv_sec = i64::from(old[0]);
    (*output).tv_nsec = old[1];
    0
}

/// musl's public time64 alias, used by Rust's SystemTime and Instant.
///
/// # Safety
/// A non-null output must point to a writable, correctly aligned timespec.
#[no_mangle]
pub unsafe extern "C" fn __clock_gettime64(
    clock: libc::clockid_t,
    output: *mut Timespec64,
) -> c_int {
    legacy_clock(clock, output)
}

/// Intercept internal musl clock reads, including timed condition-variable waits.
///
/// # Safety
/// A non-null output must point to a writable, correctly aligned timespec.
#[no_mangle]
pub unsafe extern "C" fn __clock_gettime(clock: libc::clockid_t, output: *mut Timespec64) -> c_int {
    legacy_clock(clock, output)
}
