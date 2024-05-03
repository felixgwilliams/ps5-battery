// https://stackoverflow.com/a/65393488

#[cfg(feature = "gui")]
fn main() -> std::io::Result<()> {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winres::WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/icon.ico")
            .compile()?;
    }
    Ok(())
}
#[cfg(not(feature = "gui"))]
fn main() -> std::io::Result<()> {
    Ok(())
}
