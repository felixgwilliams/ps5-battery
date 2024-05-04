// https://stackoverflow.com/a/65393488

#[cfg(feature = "gui")]
fn main() -> std::io::Result<()> {
    use std::io::{BufWriter, Write};

    let icon_png = image::io::Reader::open("assets/app-icon-128.png")?
        .decode()
        .unwrap()
        .to_rgba8();
    {
        let file = std::fs::File::create("assets/app-icon-128.rgba")?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&icon_png)?;
    }

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
