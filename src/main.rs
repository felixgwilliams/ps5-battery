#![cfg_attr(feature = "gui", windows_subsystem = "windows")]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)] // can't do anything about these
mod controllers;
mod info;

#[cfg(feature = "gui")]
mod gui;

#[cfg(not(feature = "gui"))]
mod cli;

#[cfg(feature = "gui")]
fn main() -> iced::Result {
    gui::main()
}

#[cfg(not(feature = "gui"))]
fn main() -> Result<(), anyhow::Error> {
    cli::main()
}
