#[repr(C)]
pub struct Complex {
    real: f32,
    imag: f32,
}

#[no_mangle]
pub extern "C" fn __mulsc3(a_real: f32, a_imag: f32, b_real: f32, b_imag: f32) -> Complex {
    let real = a_real * b_real - a_imag * b_imag;
    let imag = a_real * b_imag + a_imag * b_real;
    Complex { real, imag }
}
