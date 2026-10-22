//! UniKey desktop shell entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    unikey_desktop_lib::run()
}
