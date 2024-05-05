use anyhow::bail;
use hidapi::{DeviceInfo, HidApi, HidDevice, HidResult};
use std::{
    cmp::min,
    fmt::{Debug, Display},
    ops::Deref,
};

const VENDOR_ID: u16 = 0x054c;
const PRODUCT_ID_DUALSENSE: u16 = 0x0ce6;
const PRODUCT_ID_DS4: u16 = 0x05C4;
const PRODUCT_ID_DS4SLIM: u16 = 0x09CC;

const USB_LEN: usize = 64;
const BT_LEN: usize = 78;
const BUF_SIZE: usize = 100;
const REPORT_LEN: usize = 63;
const BT_EXTRA_LEN: usize = 13;

// pub fn init_device(api: &HidApi, device: &TypedDevice) -> anyhow::Result<()> {
//     let open_device = device
//         .open_device(api)
//         .or_else(|_e| bail!("Could not find dualsense"))?;

//     let mut buf = [0u8; 64];
//     buf[0] = 0x05;
//     open_device.get_feature_report(&mut buf[..])?;

//     Ok(())
// }
pub struct DeviceFilterer<'a, T: AsRef<str>> {
    pub serial_numbers: Option<&'a [T]>,
}

pub enum ControllerType {
    DualSense,
    DualShock4,
    DualShock4Slim,
}
pub struct TypedDevice<'a> {
    pub controller_type: ControllerType,
    device: &'a DeviceInfo,
    pub ready_state: ReadyState,
}

pub enum ReadyState {
    Ready,
    NotReady,
}
impl<'a> TypedDevice<'a> {
    pub fn serial_number(&self) -> Option<&str> {
        self.device.serial_number()
    }
    pub fn open_device(&self, hidapi: &HidApi) -> HidResult<HidDevice> {
        self.device.open_device(hidapi)
    }

    pub fn init_device(&mut self, api: &HidApi) -> anyhow::Result<()> {
        let open_device = self
            .open_device(api)
            .or_else(|_e| bail!("Could not find dualsense"))?;

        let mut buf = [0u8; 64];
        buf[0] = 0x05;
        open_device.get_feature_report(&mut buf[..])?;
        self.ready_state = ReadyState::Ready;

        Ok(())
    }

    pub fn make_device(device: &'a DeviceInfo) -> Option<Self> {
        let controller_type = match (device.vendor_id(), device.product_id()) {
            (VENDOR_ID, PRODUCT_ID_DUALSENSE) => ControllerType::DualSense,
            (VENDOR_ID, PRODUCT_ID_DS4) => ControllerType::DualShock4,
            (VENDOR_ID, PRODUCT_ID_DS4SLIM) => ControllerType::DualShock4Slim,
            (_, _) => return None,
        };

        Some(TypedDevice {
            controller_type,
            device,
            ready_state: ReadyState::NotReady,
        })
    }
}

impl<'a, T: AsRef<str>> DeviceFilterer<'a, T> {
    pub fn predicate(&self, device: &'a DeviceInfo) -> Option<TypedDevice<'a>> {
        let typed_device = TypedDevice::make_device(device)?;
        let Some(sn_list) = self.serial_numbers else {
            return Some(typed_device);
        };
        let dev_sn = typed_device.serial_number();

        if sn_list.iter().any(|x| Some(x.as_ref()) == dev_sn) {
            Some(typed_device)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum ChargeState {
    Discharging,
    Charging,
    Full,
    ChargingError,
    AbnormalVoltage,
    AbnormalTemp,
    Unknown,
}

pub struct Battery {
    pub state: ChargeState,
    pub level: u8,
}

pub fn read_battery_state(battery_byte: u8) -> Battery {
    let level_byte = battery_byte & 0x0F;
    let level = min(8, level_byte);
    let state_byte = battery_byte & 0xF0;

    let state = match state_byte {
        0x0 => ChargeState::Discharging,
        0x1 => ChargeState::Charging,
        0x2 => ChargeState::Full,
        0x0f => ChargeState::ChargingError,
        0x0A => ChargeState::AbnormalVoltage,
        0x0b => ChargeState::AbnormalTemp,
        _ => ChargeState::Unknown,
    };
    Battery { state, level }
}

impl Display for ChargeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlugState {
    headphones: bool,
    mic: bool,
    muted: bool,
    usb_data: bool,
    usb_power: bool,
    plugged_unk1: bool,
    plugged_dock: bool,
}

pub fn read_plug_stage(plug_byte: u8) -> PlugState {
    let headphones = (plug_byte & 0x01) > 0;
    let mic = (plug_byte & 0x02) > 0;
    let muted = (plug_byte & 0x04) > 0;
    let usb_data = (plug_byte & 0x08) > 0;
    let usb_power = (plug_byte & 0x10) > 0;
    let plugged_unk1 = (plug_byte & 0x20) > 0;
    let plugged_dock = (plug_byte & 0x40) > 0;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnType {
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

pub enum Report {
    DsBluetooth([u8; REPORT_LEN], [u8; BT_EXTRA_LEN]),
    DsUSB([u8; REPORT_LEN]),
}
impl Report {
    pub fn get_standard_report(&self) -> &[u8] {
        match self {
            Self::DsBluetooth(report, _) => report,
            Self::DsUSB(report) => report,
        }
    }
}

pub struct ReadyController {
    pub serial_number: Option<String>,
    pub type_: ControllerType,
    pub conn_type: ConnType,
    // report: Report,
    pub plug: PlugState,
    pub battery: Battery,
}
pub struct NotReadyController {
    pub serial_number: Option<String>,
    pub type_: ControllerType,
    pub conn_type: ConnType,
}

pub enum Controller {
    Ready(ReadyController),
    NotReady(NotReadyController),
}
impl Controller {
    pub fn conn_type(&self) -> ConnType {
        match self {
            Controller::NotReady(con) => con.conn_type,
            Controller::Ready(con) => con.conn_type,
        }
    }
    pub fn serial_number(&self) -> Option<&str> {
        match self {
            Controller::NotReady(con) => con.serial_number.as_deref(),
            Controller::Ready(con) => con.serial_number.as_deref(),
        }
    }
}

fn get_report(
    raw_report: [u8; BUF_SIZE],
    conn_type: ConnType,
) -> Result<Option<Report>, anyhow::Error> {
    Ok(match (conn_type, raw_report[0]) {
        (ConnType::Bluetooth, 0x31) => Some(Report::DsBluetooth(
            raw_report[2..REPORT_LEN + 2].try_into().unwrap(),
            raw_report[REPORT_LEN + 2..REPORT_LEN + BT_EXTRA_LEN + 2].try_into()?,
        )),
        (ConnType::Bluetooth, _) => None,
        (ConnType::Usb, _) => Some(Report::DsUSB(raw_report[1..REPORT_LEN + 1].try_into()?)),
    })
}

pub fn read_controller<T: Deref<Target = str>>(
    raw_report: [u8; BUF_SIZE],
    bytes_read: usize,
    serial_number: Option<T>,
) -> Result<Controller, anyhow::Error> {
    let conn_type = ConnType::try_from(bytes_read)?;

    if let Some(report) = get_report(raw_report, conn_type)? {
        let standard_report = report.get_standard_report();
        let plug = read_plug_stage(standard_report[53]);
        let battery = read_battery_state(standard_report[52]);

        Ok(Controller::Ready(ReadyController {
            serial_number: serial_number.map(|x| x.to_owned()),
            // report,
            conn_type,
            type_: ControllerType::DualSense,
            plug,
            battery,
        }))
    } else {
        Ok(Controller::NotReady(NotReadyController {
            serial_number: serial_number.map(|x| x.to_owned()),
            type_: ControllerType::DualSense,
            conn_type,
        }))
    }
}
