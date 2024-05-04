use crate::controllers::{init_device, read_controller, ConnType, Controller, DeviceFilterer};
use anyhow::bail;
use hidapi::{DeviceInfo, HidApi};
use std::{collections::HashSet, fmt::Display};
pub static BUTTON_UPDATE: &str = "Update";

fn print_ds_info<T: Display, W: std::fmt::Write>(
    str_buf: &mut W,
    id: T,
    api: &HidApi,
    device: &DeviceInfo,
    show_serial_number: bool,
) -> Result<(), anyhow::Error> {
    let open_device = device
        .open_device(api)
        .or_else(|_e| bail!("Could not find dualsense"))?;

    let mut buf = [0u8; 100];
    let bytes_read = open_device.read_timeout(&mut buf[..], 1000)?;
    // let conn_type = ConnType::try_from(bytes_read)?;
    // let report = match conn_type {
    //     ConnType::Bluetooth => &buf[2..],
    //     ConnType::Usb => &buf[1..],
    // };
    // #[cfg(not(feature = "gui"))]
    // {
    //     println!("Bytes read: {}", bytes_read);
    //     println!("{:02x?}", buf);
    // }
    // if conn_type == ConnType::Bluetooth && buf[0] != 0x31 {
    //     // bail!("Unknown Report ID {:02x}, must be 0x31", buf[0])
    //     writeln!(str_buf, "Please press {BUTTON_UPDATE} again.").unwrap();
    //     return Ok(());
    // }
    // if show_serial_number && conn_type == ConnType::Bluetooth {
    //     writeln!(
    //         str_buf,
    //         "Dualsense {} (S/N {}):",
    //         id,
    //         device.serial_number().unwrap_or("N/A")
    //     )
    //     .unwrap();
    // } else {
    //     writeln!(str_buf, "Dualsense {}:", id,).unwrap();
    // }

    // let battery_byte = report[52];
    // let plug_byte = report[53];
    // let plug_state = read_plug_stage(plug_byte);
    // let battery = read_battery_state(battery_byte);
    let controller = read_controller(buf, bytes_read, device.serial_number())?;
    if show_serial_number && controller.conn_type() == ConnType::Bluetooth {
        writeln!(
            str_buf,
            "Dualsense {} (S/N {}):",
            id,
            controller.serial_number().unwrap_or("N/A")
        )
        .unwrap();
    } else {
        writeln!(str_buf, "Dualsense {}:", id,).unwrap();
    }

    let Controller::Ready(ready_controller) = controller else {
        writeln!(str_buf, "Please press {BUTTON_UPDATE} again.").unwrap();
        return Ok(());
    };

    let batt_level = ready_controller.battery.level as f64 / 8.0f64;

    writeln!(str_buf, "Battery Level: {}%", batt_level * 100.0).unwrap();
    writeln!(str_buf, "Battery Status {}", ready_controller.battery.state).unwrap();
    writeln!(str_buf, "Plug Status: {}", ready_controller.plug).unwrap();
    Ok(())
}

pub fn print_all_ds_info<T: AsRef<str>, W: std::fmt::Write>(
    buf: &mut W,
    api: &HidApi,
    device_filterer: &DeviceFilterer<T>,
    show_serial_number: bool,
    init_sns: &HashSet<String>,
) -> Result<(), anyhow::Error> {
    let mut device_found = false;
    for (i, device) in api
        .device_list()
        .filter(|dev| device_filterer.predicate(dev))
        .enumerate()
    {
        device_found = true;
        if !device
            .serial_number()
            .is_some_and(|sn| init_sns.contains(sn))
        {
            init_device(api, device)?;
        }
        print_ds_info(buf, i + 1, api, device, show_serial_number)?;
    }
    if !device_found {
        writeln!(buf, "No Dualsenses Found").unwrap();
    }
    Ok(())
}
