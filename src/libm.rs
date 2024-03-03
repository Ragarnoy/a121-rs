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

#[no_mangle]
pub extern "C" fn cexpf(f: f32) -> f32 {
    libm::expf(f)
}

#[no_mangle]
pub extern "C" fn cabsf(f: f32) -> f32 {
    libm::fabsf(f)
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
