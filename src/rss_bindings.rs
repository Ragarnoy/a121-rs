#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(dead_code)]

use core::concat;
use core::env;
use core::include;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
