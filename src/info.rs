use crate::controllers::{ConnType, DeviceFilterer, TypedDevice};
use hidapi::HidApi;
use std::{collections::HashSet, fmt::Display};
pub static BUTTON_UPDATE: &str = "Update";

fn print_ds_info<T: Display, W: std::fmt::Write>(
    str_buf: &mut W,
    id: T,
    api: &HidApi,
    device: &mut TypedDevice,
    show_serial_number: bool,
) -> Result<(), anyhow::Error> {
    let controller = device.read_controller(api)?;
    if show_serial_number && controller.conn_type == ConnType::Bluetooth {
        writeln!(
            str_buf,
            "{} {} (S/N {}):",
            device.controller_type.name(),
            id,
            controller.serial_number.as_deref().unwrap_or("N/A")
        )
        .unwrap();
    } else {
        writeln!(str_buf, "{} {}:", device.controller_type.name(), id,).unwrap();
    }

    let Some(status) = controller.status else {
        writeln!(str_buf, "Please press {BUTTON_UPDATE} again.").unwrap();
        return Ok(());
    };

    let batt_level = status.battery.level;

    writeln!(str_buf, "Battery Level: {:.00}%", batt_level * 100.0).unwrap();
    writeln!(str_buf, "Battery Status {}", status.battery.state).unwrap();
    writeln!(str_buf, "Plug Status: {}", status.plug).unwrap();
    Ok(())
}

#[allow(clippy::module_name_repetitions)]
pub fn print_all_ds_info<T: AsRef<str>, W: std::fmt::Write>(
    buf: &mut W,
    api: &HidApi,
    device_filterer: &DeviceFilterer<T>,
    show_serial_number: bool,
    init_sns: &HashSet<String>,
) -> Result<(), anyhow::Error> {
    let mut device_found = false;
    for (i, mut device) in api
        .device_list()
        .filter_map(|dev| device_filterer.predicate(dev))
        .enumerate()
    {
        device_found = true;
        if !device
            .serial_number()
            .is_some_and(|sn| init_sns.contains(sn))
        {
            device.init_device(api)?;
        }
        print_ds_info(buf, i + 1, api, &mut device, show_serial_number)?;
    }
    if !device_found {
        writeln!(buf, "No Gamepads Found").unwrap();
    }
    Ok(())
}
