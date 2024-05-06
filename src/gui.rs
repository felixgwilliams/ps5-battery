// use super::*;
use crate::controllers::DeviceFilterer;
use crate::info::{print_all_ds_info, BUTTON_UPDATE};

use hidapi::HidApi;
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{window, Element, Font, Length, Sandbox, Settings, Size};
use std::collections::HashSet;

static BUTTON_CLEAR: &str = "Clear";
static CHECK_SHOW_SN: &str = "Show BT S/N";
const SHOW_SN_DEFAULT: bool = false;
pub const ICON: &[u8] = include_bytes!("../assets/app-icon-128.rgba");

struct DualSenseStatus {
    text: String,
    show_sn: bool,
    init_sns: HashSet<String>,
}
#[derive(Debug, Clone, Copy)]
enum Message {
    Clear,
    GetStatus,
    SNToggled(bool),
}

impl Sandbox for DualSenseStatus {
    type Message = Message;
    fn new() -> Self {
        let mut init_sns = HashSet::new();
        let api = HidApi::new().unwrap();
        let device_filterer: DeviceFilterer<'_, &str> = DeviceFilterer {
            serial_numbers: None,
        };
        for mut device in api
            .device_list()
            .filter_map(|dev| device_filterer.predicate(dev))
        {
            device.init_device(&api).expect("Could not init device.");
            if let Some(sn) = device.serial_number() {
                init_sns.insert(sn.to_owned());
            }
        }
        Self {
            text: String::new(),
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
                .unwrap();
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
            icon: iced::window::icon::from_rgba(ICON.to_owned(), 128, 128).ok(),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
