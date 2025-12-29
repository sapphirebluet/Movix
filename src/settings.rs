use std::path::PathBuf;

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};
use serde::{Deserialize, Serialize};

use crate::media::{BACKGROUND_BLACK, NETFLIX_RED, TEXT_GRAY, TEXT_WHITE};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub api_key: String,
    pub language: String,
}

impl AppSettings {
    pub fn config_path() -> Option<PathBuf> {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("movix")
                .join("config.json")
        })
    }

    pub fn load() -> Option<Self> {
        let path = Self::config_path()?;
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config path")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    pub fn is_valid(&self) -> bool {
        !self.api_key.trim().is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum SetupMessage {
    ApiKeyChanged(String),
    LanguageChanged(String),
    Submit,
}

pub struct SetupPage {
    pub api_key: String,
    pub language: String,
    pub error: Option<String>,
}

impl Default for SetupPage {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            language: String::from("en-US"),
            error: None,
        }
    }
}

impl SetupPage {
    pub fn update(&mut self, message: SetupMessage) -> Option<AppSettings> {
        match message {
            SetupMessage::ApiKeyChanged(key) => {
                self.api_key = key;
                self.error = None;
                None
            }
            SetupMessage::LanguageChanged(lang) => {
                self.language = lang;
                None
            }
            SetupMessage::Submit => {
                if self.api_key.trim().is_empty() {
                    self.error = Some(String::from("API key is required"));
                    return None;
                }
                let settings = AppSettings {
                    api_key: self.api_key.trim().to_string(),
                    language: if self.language.trim().is_empty() {
                        String::from("en-US")
                    } else {
                        self.language.trim().to_string()
                    },
                };
                if let Err(e) = settings.save() {
                    self.error = Some(format!("Failed to save: {}", e));
                    return None;
                }
                Some(settings)
            }
        }
    }

    pub fn view(&self) -> Element<'_, SetupMessage> {
        let logo = text("MOVIX")
            .size(48)
            .color(NETFLIX_RED)
            .font(iced::Font::with_name("sans-serif"));

        let title = text("Welcome to Movix").size(28).color(TEXT_WHITE);
        let subtitle = text("Configure your TMDB API settings to get started")
            .size(14)
            .color(TEXT_GRAY);

        let api_label = text("TMDB API Key").size(14).color(TEXT_WHITE);
        let api_hint = text("Get your free API key at themoviedb.org/settings/api")
            .size(12)
            .color(TEXT_GRAY);
        let api_input = text_input("Enter your TMDB API key...", &self.api_key)
            .on_input(SetupMessage::ApiKeyChanged)
            .on_submit(SetupMessage::Submit)
            .padding(12)
            .size(14)
            .width(Length::Fill);

        let lang_label = text("Language").size(14).color(TEXT_WHITE);
        let lang_hint = text("Examples: en-US, de-DE, fr-FR, es-ES")
            .size(12)
            .color(TEXT_GRAY);
        let lang_input = text_input("en-US", &self.language)
            .on_input(SetupMessage::LanguageChanged)
            .on_submit(SetupMessage::Submit)
            .padding(12)
            .size(14)
            .width(Length::Fill);

        let submit_button = button(text("Get Started").size(16).color(TEXT_WHITE))
            .padding([12, 32])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => iced::Color::from_rgb(0.7, 0.02, 0.06),
                    _ => NETFLIX_RED,
                };
                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    text_color: TEXT_WHITE,
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                }
            })
            .on_press(SetupMessage::Submit);

        let error_text = if let Some(ref err) = self.error {
            text(err).size(14).color(NETFLIX_RED)
        } else {
            text("").size(14)
        };

        let spacer = || Space::new().height(16);
        let small_spacer = || Space::new().height(4);

        let form = column![
            logo,
            spacer(),
            title,
            small_spacer(),
            subtitle,
            spacer(),
            spacer(),
            api_label,
            small_spacer(),
            api_hint,
            small_spacer(),
            api_input,
            spacer(),
            lang_label,
            small_spacer(),
            lang_hint,
            small_spacer(),
            lang_input,
            spacer(),
            error_text,
            small_spacer(),
            row![submit_button].width(Length::Fill),
        ]
        .width(Length::Fixed(400.0))
        .align_x(Alignment::Start);

        container(form)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(BACKGROUND_BLACK)),
                ..Default::default()
            })
            .into()
    }
}
