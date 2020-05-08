// FIXME: make ALL the functions below return Result-wrapped values
#![feature(new_uninit)]
#![feature(proc_macro_hygiene)]
//#![feature(try_reserve)]

//#[macro_use]
//extern crate casey;

#[macro_use]
extern crate bitflags;

use aftereffects_sys as ae_sys;
use num_enum::{IntoPrimitive, UnsafeFromPrimitive};
use std::{cell::RefCell, ops::Add};

thread_local!(
    pub static PICA_BASIC_SUITE: RefCell<*const ae_sys::SPBasicSuite> =
        RefCell::new(std::ptr::null_mut())
);

#[inline]
pub fn borrow_pica_basic_as_ptr() -> *const ae_sys::SPBasicSuite {
    let mut pica_basic_ptr: *const ae_sys::SPBasicSuite = std::ptr::null();

    PICA_BASIC_SUITE.with(|pica_basic_ptr_cell| {
        pica_basic_ptr = *pica_basic_ptr_cell.borrow();
    });

    pica_basic_ptr
}

/// This lets us access a thread-local version of the PicaBasic
/// Suite. Whenever we gen a new SPBasic_Suite from Ae somehow,
/// we create a PicaBasicSuite::new from it and use that to initialize
/// access to any suites.
/// When we leave scope, drop() ic alled automatically and restores the
/// previous value to our thread local storage so the caller
/// can continue using their pointer to the suite.
///
/// FIXME: Is this really neccessary? Check if the pointer is always the
///        same and if so, confirm with Adobe we can get rid of it.
pub struct PicaBasicSuite {
    previous_pica_basic_suite_ptr: *const ae_sys::SPBasicSuite,
}

impl PicaBasicSuite {
    fn set(pica_basic_suite_ptr: *const ae_sys::SPBasicSuite) -> *const ae_sys::SPBasicSuite {
        let mut previous_pica_basic_suite_ptr: *const ae_sys::SPBasicSuite = std::ptr::null();

        PICA_BASIC_SUITE.with(|pica_basic_ptr_cell| {
            previous_pica_basic_suite_ptr = pica_basic_ptr_cell.replace(pica_basic_suite_ptr);
        });

        previous_pica_basic_suite_ptr
    }

    #[inline]
    pub fn from_pr_in_data_raw(in_data_ptr: *const ae_sys::PR_InData) -> Self {
        Self {
            previous_pica_basic_suite_ptr: PicaBasicSuite::set(unsafe { *in_data_ptr }.pica_basicP),
        }
    }

    #[inline]
    pub fn from_pr_in_data(in_data_handle: pr::InDataHandle) -> Self {
        Self {
            previous_pica_basic_suite_ptr: PicaBasicSuite::set(unsafe { *in_data_handle.as_ptr() }.pica_basicP),
        }
    }

    #[inline]
    pub fn from_pf_in_data_raw(in_data_ptr: *const ae_sys::PF_InData) -> Self {
        Self {
            previous_pica_basic_suite_ptr: PicaBasicSuite::set(unsafe { *in_data_ptr }.pica_basicP),
        }
    }

    #[inline]
    pub fn from_sp_basic_suite_raw(pica_basic_suite_ptr: *const ae_sys::SPBasicSuite) -> Self {
        Self {
            previous_pica_basic_suite_ptr: PicaBasicSuite::set(pica_basic_suite_ptr),
        }
    }
}

impl Drop for PicaBasicSuite {
    #[inline]
    fn drop(&mut self) {
        PICA_BASIC_SUITE.with(|pica_basic_ptr_cell| {
            pica_basic_ptr_cell.replace(self.previous_pica_basic_suite_ptr);
        });
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, IntoPrimitive, UnsafeFromPrimitive)]
#[repr(i32)]
pub enum Error {
    Generic = ae_sys::A_Err_GENERIC as i32,
    Struct = ae_sys::A_Err_STRUCT as i32,
    Parameter = ae_sys::A_Err_PARAMETER as i32,
    Alloc = ae_sys::A_Err_ALLOC as i32,
    // Some calls can only be used on UI (Main) or Render threads.
    // Also, calls back to Ae can only be made from the same thread Ae
    // called you on.
    WrongThread = ae_sys::A_Err_WRONG_THREAD as i32,
    // An attempt was made to write to a read only copy of an AE
    // project. Project changes must originate in the UI/Main thread.
    ConstProjectModification = ae_sys::A_Err_CONST_PROJECT_MODIFICATION as i32,
    // Acquire suite failed on a required suite.
    MissingSuite = ae_sys::A_Err_MISSING_SUITE as i32,
}

/* FIXME uncomment this once TryReserve() becomes stable in nightly
impl From<std::collections::TryReserveError> for Error {
    fn from(try_reserve_err: std::collections::TryReserveError) -> Self {
        Error::Alloc
    }
}*/

struct AeError(pub i32);

impl From<Result<(), Error>> for AeError {
    #[inline]
    fn from(result: Result<(), Error>) -> Self {
        match result {
            Ok(()) => AeError(0),
            Err(e) => AeError(e.into()),
        }
    }
}

impl From<AeError> for i32 {
    #[inline]
    fn from(ae_error: AeError) -> Self {
        ae_error.0
    }
}

#[macro_use]
pub mod macros;
pub use macros::*;

pub mod aegp;
pub use aegp::*;
pub mod aeio;
pub use aeio::*;
pub mod pf;
pub use pf::*;
pub mod pr;
pub use pr::*;

pub struct Matrix4([[f64; 4]; 4]);

impl Matrix4 {
    #[inline]
    fn as_slice(&self) -> &[f64] {
        unsafe { std::slice::from_raw_parts(self.0.as_ptr() as _, 16) }
    }
}

#[cfg(feature = "algebra-nalgebra")]
impl From<Matrix4> for nalgebra::Matrix4<f64> {
    #[inline]
    fn from(m: Matrix4) -> Self {
        nalgebra::Matrix4::<f64>::from_row_slice(m.as_slice())
    }
}

#[cfg(feature = "algebra-nalgebra")]
#[test]
fn test_from() {
    let m = Matrix4 {
        0: [[0., 0., 0., 0.], [0., 0., 0., 0.], [0., 0., 0., 0.], [0., 0., 0., 0.]],
    };
    let _matrix = nalgebra::Matrix4::<f64>::from(m);
}

pub type Color = ae_sys::A_Color;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct Time {
    #[inline]
    pub value: ae_sys::A_long,
    pub scale: ae_sys::A_u_long,
}

impl From<Time> for f64 {
    #[inline]
    fn from(time: Time) -> Self {
        time.value as f64 / time.scale as f64
    }
}

impl From<Time> for f32 {
    #[inline]
    fn from(time: Time) -> Self {
        (time.value as f64 / time.scale as f64) as f32
    }
}

// Next bit (std::ops::Add) ported from aeutility.cpp
// Rust version is so much shorter & cleaner!

// Euclid's two-thousand-year-old algorithm for finding the
// greatest common divisor. Copied non-recursive version from
// Rust docs.
#[inline]
fn greatest_common_divisor(x: u32, y: u32) -> u32 {
    let mut x = x;
    let mut y = y;
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    x
}

// Calculates the Option-wrapped sum of two Times. If no common integer
// demoninator can be found, None is returned.
#[inline]
fn add_time_lossless(time1: Time, time2: Time) -> Option<Time> {
    if (time1.scale == 0) || (time2.scale == 0) {
        return None;
    }

    let gcd = {
        if time1.scale == time2.scale {
            time1.scale
        } else {
            greatest_common_divisor(time1.scale, time2.scale)
        }
    };

    let value1 = time1.value.checked_mul(time2.scale.checked_div(gcd)? as i32)?;
    let value2 = time2.value.checked_mul(time1.scale.checked_div(gcd)? as i32)?;

    Some(Time {
        value: value1.checked_add(value2)?,
        scale: time2.scale.checked_mul(time1.scale.checked_div(gcd)?)?,
    })
}

// Calculates the sum of two Times using FP math.
#[inline]
fn add_time_lossy(time1: Time, time2: Time) -> Time {
    let time = (time1.value as f64 / time1.scale as f64) + (time2.value as f64 / time2.scale as f64);

    let num_bits = time.log2() as usize;
    let scale: u32 = 1u32 << (30 - num_bits);

    Time {
        value: (time * scale as f64) as i32,
        scale,
    }
}

// Calculates the sum of two Times, lossless if possible.
// FIXME: is it worth going the lossless route at all???
impl Add<Time> for Time {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        match add_time_lossless(self, rhs) {
            Some(time) => time,
            None => add_time_lossy(self, rhs),
        }
    }
}

impl From<Time> for ae_sys::A_Time {
    #[inline]
    fn from(time: Time) -> Self {
        Self {
            value: time.value,
            scale: time.scale,
        }
    }
}

// Note that this has a different ordering
// of values than Legacy_Rect!!!
#[derive(Debug, Copy, Clone, Hash)]
#[repr(C)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// Note that this has a different ordering
// of values than Rect!!!
#[derive(Debug, Copy, Clone, Hash)]
#[repr(C)]
pub struct LegacyRect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

#[derive(Debug, Copy, Clone, Hash)]
#[repr(C)]
pub struct Ratio {
    pub num: ae_sys::A_long,
    pub den: ae_sys::A_u_long,
}

impl From<Ratio> for ae_sys::A_Ratio {
    #[inline]
    fn from(ratio: Ratio) -> Self {
        Self {
            num: ratio.num,
            den: ratio.den,
        }
    }
}

// This is confusing: for some structs Ae expects the caller to
// manage the memory and for others it doesn't (the caller only
// deals with a pointer that gets dereferenced for actually
// calling into the suite). In this case the struct ends
// with a 'H' (for handle).
// When the struct misses the trailing 'H', Ae does expect us to
// manage the memory. We then use a Box<T>.
pub struct PicaBasicSuiteHandle {
    #[inline]
    pica_basic_suite_ptr: *const ae_sys::SPBasicSuite,
}

impl PicaBasicSuiteHandle {
    #[inline]
    pub fn from_raw(pica_basic_suite_ptr: *const ae_sys::SPBasicSuite) -> PicaBasicSuiteHandle {
        /*if pica_basic_suite_ptr == ptr::null() {
            panic!()
        }*/
        PicaBasicSuiteHandle { pica_basic_suite_ptr }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const ae_sys::SPBasicSuite {
        self.pica_basic_suite_ptr
    }
}

pub trait Suite: Drop {
    fn new() -> Result<Self, Error>
    where
        Self: Sized;

    fn from_raw(pica_basic_suite_raw_ptr: *const crate::ae_sys::SPBasicSuite) -> Result<Self, Error>
    where
        Self: Sized;
}
