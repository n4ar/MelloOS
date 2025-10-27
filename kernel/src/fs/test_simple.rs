//! Simple test to check syscalls module

use crate::fs::syscalls::sys_open;

pub fn test_simple() {
    let _result = sys_open(0, 0, 0);
}