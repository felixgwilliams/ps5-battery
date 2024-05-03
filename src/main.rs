#![cfg_attr(feature = "gui", windows_subsystem = "windows")]
use anyhow::bail;
use clap::{command, Parser};
use hidapi::{DeviceInfo, HidApi};

use std::collections::HashSet;
use std::{
    cmp::min,
    fmt::{Debug, Display},
};

const VENDOR_ID: u16 = 0x054c;
const PRODUCT_ID: u16 = 0x0ce6;
const USB_LEN: usize = 64;
const BT_LEN: usize = 78;

static BUTTON_UPDATE: &str = "Update";

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

#[cfg(feature = "gui")]
mod gui {
    use super::*;
    use iced::widget::{button, checkbox, column, container, row, text};
    use iced::{window, Element, Font, Length, Sandbox, Settings, Size};
    static BUTTON_CLEAR: &str = "Clear";
    static CHECK_SHOW_SN: &str = "Show BT S/N";
    const SHOW_SN_DEFAULT: bool = false;
    pub const ICON: &[u8] = include_bytes!("../assets/icon.ico");
    #[cfg(feature = "gui")]
    struct DualSenseStatus {
        text: String,
        show_sn: bool,
        init_sns: HashSet<String>,
    }
    #[cfg(feature = "gui")]
    #[derive(Debug, Clone, Copy)]
    enum Message {
        Clear,
        GetStatus,
        SNToggled(bool),
    }
    #[cfg(feature = "gui")]
    impl Sandbox for DualSenseStatus {
        type Message = Message;
        fn new() -> Self {
            let mut init_sns = HashSet::new();
            let api = HidApi::new().unwrap();
            let device_filterer: DeviceFilterer<'_, &str> = DeviceFilterer {
                serial_numbers: None,
            };
            for device in api
                .device_list()
                .filter(|dev| device_filterer.predicate(dev))
            {
                init_device(&api, device).expect("Could not init device.");
                if let Some(sn) = device.serial_number() {
                    init_sns.insert(sn.to_owned());
                }
            }
            DualSenseStatus {
                text: "".to_owned(),
                show_sn: SHOW_SN_DEFAULT,
                init_sns,
            }
        }
        fn title(&self) -> String {
            String::from("PS5 battery")
        }

        fn update(&mut self, message: Message) {
            match message {
                Message::Clear => self.text.clear(),
                Message::GetStatus => {
                    self.text.clear();
                    let api = HidApi::new().unwrap();
                    let device_filterer: DeviceFilterer<'_, &str> = DeviceFilterer {
                        serial_numbers: None,
                    };
                    print_all_ds_info(
                        &mut self.text,
                        &api,
                        &device_filterer,
                        self.show_sn,
                        &self.init_sns,
                    )
                    .unwrap()
                }
                Message::SNToggled(show_sn) => self.show_sn = show_sn,
            }
        }
        fn view(&self) -> Element<Message> {
            let stuff = column![
                text(&self.text).font(Font::MONOSPACE),
                row![
                    button(BUTTON_UPDATE)
                        .on_press(Message::GetStatus)
                        .padding(10),
                    button(BUTTON_CLEAR).on_press(Message::Clear).padding(10),
                    checkbox(CHECK_SHOW_SN, self.show_sn).on_toggle(Message::SNToggled)
                ]
                .spacing(20)
            ]
            .spacing(20);
            container(stuff)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                // .padding(20)
                .into()
        }
    }
    pub fn main() -> iced::Result {
        DualSenseStatus::run(Settings {
            window: window::Settings {
                size: Size::new(400.0, 225.0),
                icon: iced::window::icon::from_file_data(ICON, Some(image::ImageFormat::Ico)).ok(),
                ..window::Settings::default()
            },
            ..Settings::default()
        })
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    let conn_type = ConnType::try_from(bytes_read)?;
    let report = match conn_type {
        ConnType::Bluetooth => &buf[2..],
        ConnType::Usb => &buf[1..],
    };
    #[cfg(not(feature = "gui"))]
    {
        println!("Bytes read: {}", bytes_read);
        println!("{:02x?}", buf);
    }
    if conn_type == ConnType::Bluetooth && buf[0] != 0x31 {
        // bail!("Unknown Report ID {:02x}, must be 0x31", buf[0])
        writeln!(str_buf, "Please press {BUTTON_UPDATE} again.").unwrap();
        return Ok(());
    }
    if show_serial_number && conn_type == ConnType::Bluetooth {
        writeln!(
            str_buf,
            "Dualsense {} (S/N {}):",
            id,
            device.serial_number().unwrap_or("N/A")
        )
        .unwrap();
    } else {
        writeln!(str_buf, "Dualsense {}:", id,).unwrap();
    }

    let battery_0 = report[52];
    let battery_1 = report[53];
    let plug_state: PlugState = battery_1.into();

    let battery_level_raw = min(8, battery_0 & 0x0f);

    let batt_level = battery_level_raw as f64 / 8.0f64;
    let battery_status: BatteryState = ((battery_0 & 0xF0) >> 4).into();

    writeln!(str_buf, "Battery Level: {}%", batt_level * 100.0).unwrap();
    writeln!(str_buf, "Battery Status {}", battery_status).unwrap();
    writeln!(str_buf, "Plug Status: {}", plug_state).unwrap();
    Ok(())
}

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

struct DeviceFilterer<'a, T: AsRef<str>> {
    serial_numbers: Option<&'a [T]>,
}
impl<'a, T: AsRef<str>> DeviceFilterer<'a, T> {
    fn predicate(&self, device: &DeviceInfo) -> bool {
        let is_dualsense = device.vendor_id() == VENDOR_ID && device.product_id() == PRODUCT_ID;
        if !is_dualsense {
            return false;
        }
        match (self.serial_numbers, device.serial_number()) {
            (Some(sn_list), Some(dev_sn)) => sn_list.iter().any(|x| x.as_ref() == dev_sn),
            (Some(_sn_list), None) => false,
            (None, _) => is_dualsense,
        }
    }
}

fn print_all_ds_info<T: AsRef<str>, W: std::fmt::Write>(
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

#[cfg(not(feature = "gui"))]
fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    let api = HidApi::new()?;
    let device_filterer = DeviceFilterer {
        serial_numbers: cli.select.as_deref(),
    };
    let mut buf = String::new();
    print_all_ds_info(
        &mut buf,
        &api,
        &device_filterer,
        cli.show_serial_number,
        &HashSet::new(),
    )?;
    println!("{}", buf);
    Ok(())
}

fn init_device(api: &HidApi, device: &DeviceInfo) -> anyhow::Result<()> {
    let open_device = device
        .open_device(api)
        .or_else(|_e| bail!("Could not find dualsense"))?;

    let mut buf = [0u8; 64];
    buf[0] = 0x05;
    open_device.get_feature_report(&mut buf[..])?;

    Ok(())
}
#[cfg(feature = "gui")]
fn main() -> iced::Result {
    gui::main()
}
