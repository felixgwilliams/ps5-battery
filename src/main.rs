use anyhow::bail;
use std::{
    cmp::min,
    fmt::{Debug, Display},
};

use hidapi::HidApi;

const VENDOR_ID: u16 = 0x054c;
const PRODUCT_ID: u16 = 0x0ce6;
const USB_LEN: usize = 64;
const BT_LEN: usize = 78;

#[derive(Debug)]
enum BatteryState {
    Discharging,
    Charging,
    Full,
    ChargingError,
    AbnormalVoltage,
    AbnormalTemp,
    Unknown,
}

impl From<u8> for BatteryState {
    fn from(value: u8) -> Self {
        match value {
            0x0 => Self::Discharging,
            0x1 => Self::Charging,
            0x2 => Self::Full,
            0x0f => Self::ChargingError,
            0x0A => Self::AbnormalVoltage,
            0x0b => Self::AbnormalTemp,
            _ => Self::Unknown,
        }
    }
}

impl Display for BatteryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PlugState {
    headphones: bool,
    mic: bool,
    muted: bool,
    usb_data: bool,
    usb_power: bool,
    plugged_unk1: bool,
    plugged_dock: bool,
}

impl From<u8> for PlugState {
    fn from(value: u8) -> Self {
        let headphones = (value & 0x01) > 0;
        let mic = (value & 0x02) > 0;
        let muted = (value & 0x04) > 0;
        let usb_data = (value & 0x08) > 0;
        let usb_power = (value & 0x10) > 0;
        let plugged_unk1 = (value & 0x20) > 0;
        let plugged_dock = (value & 0x40) > 0;
        PlugState {
            headphones,
            mic,
            muted,
            usb_data,
            usb_power,
            plugged_unk1,
            plugged_dock,
        }
    }
}

impl Display for PlugState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = vec![];
        if self.headphones {
            out.push("headphones plugged");
        }
        if self.mic {
            out.push("mic plugged");
        }
        if self.muted {
            out.push("mic muted");
        }
        if self.usb_data {
            out.push("data usb plugged");
        }
        if self.usb_power {
            out.push("power usb plugged");
        }
        if self.plugged_unk1 || self.plugged_dock {
            out.push("power docked");
        }
        if out.is_empty() {
            write!(f, "None")
        } else {
            write!(f, "{}", out.join("; "))
        }
    }
}

enum ConnType {
    Bluetooth,
    Usb,
}

impl TryFrom<usize> for ConnType {
    type Error = anyhow::Error;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            USB_LEN => Ok(ConnType::Usb),
            BT_LEN => Ok(ConnType::Bluetooth),
            _ => {
                bail!("Unknown report length")
            }
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let api = HidApi::new()?;

    for (i, device) in api
        .device_list()
        .filter(|dev| dev.vendor_id() == VENDOR_ID && dev.product_id() == PRODUCT_ID)
        .enumerate()
    {
        println!(
            "Dualsense {} (S/N {}):",
            i + 1,
            device.serial_number().unwrap_or("N/A")
        );
        let open_device = device
            .open_device(&api)
            .or_else(|_e| bail!("Could not find dualsense"))?;

        let mut buf = [0u8; 64];
        buf[0] = 0x05;
        open_device.get_feature_report(&mut buf[..])?;

        let mut buf = [0u8; 100];
        let bytes_read = open_device.read_timeout(&mut buf[..], 1000)?;
        if buf[0] != 0x31 {
            bail!("Unknown Report ID {:02x}, must be 0x31", buf[0])
        }
        let conn_type = ConnType::try_from(bytes_read)?;
        let report = match conn_type {
            ConnType::Bluetooth => &buf[2..],
            ConnType::Usb => &buf[1..],
        };

        let battery_0 = report[52];
        let battery_1 = report[53];
        let plug_state: PlugState = battery_1.into();

        let battery_level_raw = min(8, battery_0 & 0x0f);

        let batt_level = battery_level_raw as f64 / 8.0f64;
        let battery_status: BatteryState = ((battery_0 & 0xF0) >> 4).into();

        println!("Battery Level: {}%", batt_level * 100.0);
        println!("Battery Status {}", battery_status);
        println!("Plug Status: {}", plug_state);
    }
    Ok(())
}