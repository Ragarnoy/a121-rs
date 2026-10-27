use num::Complex;

#[no_mangle]
pub extern "C" fn cosf(f: f32) -> f32 {
    libm::cosf(f)
}

#[no_mangle]
pub extern "C" fn sinf(f: f32) -> f32 {
    libm::sinf(f)
}

#[no_mangle]
pub extern "C" fn roundf(f: f32) -> f32 {
    libm::roundf(f)
}

#[no_mangle]
pub extern "C" fn sqrtf(f: f32) -> f32 {
    libm::sqrtf(f)
}

#[no_mangle]
pub extern "C" fn powf(f: f32, g: f32) -> f32 {
    libm::powf(f, g)
}

// `float complex` has the same layout and calling convention as `Complex<f32>` (`repr(C)`)
#[no_mangle]
pub extern "C" fn cexpf(z: Complex<f32>) -> Complex<f32> {
    let r = libm::expf(z.re);
    Complex::new(r * libm::cosf(z.im), r * libm::sinf(z.im))
}

#[no_mangle]
pub extern "C" fn cabsf(z: Complex<f32>) -> f32 {
    libm::hypotf(z.re, z.im)
}

#[no_mangle]
pub extern "C" fn atanf(f: f32) -> f32 {
    libm::atanf(f)
}

#[no_mangle]
pub extern "C" fn floorf(f: f32) -> f32 {
    libm::floorf(f)
}

#[no_mangle]
pub extern "C" fn log10f(f: f32) -> f32 {
    libm::log10f(f)
}

#[no_mangle]
pub extern "C" fn exp2f(f: f32) -> f32 {
    libm::exp2f(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cabsf_is_magnitude() {
        assert_eq!(cabsf(Complex::new(3.0, 4.0)), 5.0);
        assert_eq!(cabsf(Complex::new(-3.0, 0.0)), 3.0);
    }

    #[test]
    fn cexpf_matches_euler() {
        let z = cexpf(Complex::new(0.0, core::f32::consts::FRAC_PI_2));
        assert!(z.re.abs() < 1e-6);
        assert!((z.im - 1.0).abs() < 1e-6);
        let z = cexpf(Complex::new(1.0, 0.0));
        assert!((z.re - core::f32::consts::E).abs() < 1e-6);
        assert_eq!(z.im, 0.0);
    }
}
