// Pas de console en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rez_lib::run()
}
