#![cfg_attr(feature = "gui", windows_subsystem = "windows")]

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
