#[cfg(feature = "cli")]
use clap::{command, Parser};
#[cfg(feature = "cli")]
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Show serial number for controllers.
    #[arg(long, short)]
    pub show_serial_number: bool,

    /// Select particular serial number.
    #[arg(long, value_delimiter = ',')]
    pub select: Option<Vec<String>>,
}
#[cfg(feature = "cli")]
fn get_cli_opts() -> (Option<Vec<String>>, bool) {
    let cli = Cli::parse();

    (cli.select, cli.show_serial_number)
}

#[cfg(not(feature = "cli"))]
fn get_cli_opts() -> (Option<Vec<String>>, bool) {
    (None, false)
}

pub fn main() -> Result<(), anyhow::Error> {
    use std::collections::HashSet;

    use crate::controllers::DeviceFilterer;
    use hidapi::HidApi;

    use crate::info::print_all_ds_info;

    let api = HidApi::new()?;
    let (serial_numbers, show_serial_number) = get_cli_opts();
    let device_filterer = DeviceFilterer {
        serial_numbers: serial_numbers.as_deref(),
    };

    let mut buf = String::new();
    print_all_ds_info(
        &mut buf,
        &api,
        &device_filterer,
        show_serial_number,
        &HashSet::new(),
    )?;
    println!("{}", buf);
    Ok(())
}
