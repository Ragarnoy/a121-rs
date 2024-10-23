use num::Complex;

use a121_sys::{
    acc_int16_complex_t, acc_processing_meter_to_points, acc_processing_points_to_meter,
};

#[derive(Debug, Clone)]
pub struct AccComplex {
    inner: acc_int16_complex_t,
}

impl AccComplex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `AccComplex` instance from a raw pointer.
    ///
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer.
    /// The caller must ensure that the pointer is valid and points to a properly initialized `acc_int16_complex_t` struct.
    pub unsafe fn from_ptr(ptr: *const acc_int16_complex_t) -> Self {
        Self { inner: *ptr }
    }

    /// Returns a mutable pointer to the inner `acc_int16_complex_t` struct.
    /// # Safety
    /// This function is unsafe because it returns a raw pointer.
    pub unsafe fn mut_ptr(&mut self) -> *mut acc_int16_complex_t {
        &mut self.inner
    }

    pub fn ptr(&self) -> *const acc_int16_complex_t {
        &self.inner
    }
}

impl Default for AccComplex {
    fn default() -> Self {
        Self {
            inner: acc_int16_complex_t { real: 0, imag: 0 },
        }
    }
}

impl From<AccComplex> for Complex<i16> {
    fn from(acc_complex: AccComplex) -> Self {
        Complex::new(acc_complex.inner.real, acc_complex.inner.imag)
    }
}

impl From<Complex<i16>> for AccComplex {
    fn from(complex: Complex<i16>) -> Self {
        Self {
            inner: acc_int16_complex_t {
                real: complex.re,
                imag: complex.im,
            },
        }
    }
}

#[derive(Default)]
pub struct Points {
    pub points: i32,
}

impl Points {
    pub fn new(points: i32) -> Self {
        Self { points }
    }

    pub fn to_meters(&self) -> f32 {
        unsafe { acc_processing_points_to_meter(self.points) }
    }

    pub fn meters_to_points(meters: f32) -> Self {
        let points = unsafe { acc_processing_meter_to_points(meters) };
        Self { points }
    }
}
