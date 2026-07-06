//! Transparent re-export of `libc`, plus the handful of C type aliases and errno
//! constants that `libc` omits on `wasm32-unknown-unknown` (the OS-less target).
//! On every other target this is just `pub use libc::*`.
#![no_std]

pub use libc::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm_unknown {
    // C integer/pointer aliases wasi-libc defines but the OS-less libc does not.
    pub type ptrdiff_t = isize;
    pub type intptr_t = isize;
    pub type uintptr_t = usize;
    pub type off_t = i64;

    // errno constants rav1d references as opaque error codes (Linux values).
    // c_int == i32 on every Rust target; use the concrete type since the OS-less
    // libc doesn't re-export c_int.
    pub const EAGAIN: i32 = 11;
    pub const EIO: i32 = 5;
    pub const EINVAL: i32 = 22;
    pub const ENOENT: i32 = 2;
    pub const ENOMEM: i32 = 12;
    pub const ENOPROTOOPT: i32 = 92;
    pub const ERANGE: i32 = 34;
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use wasm_unknown::*;
