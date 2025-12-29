mod cards;
mod components;
mod detail_handlers;
mod detail_popup;
mod detail_sections;
mod handlers;
mod hero;
mod media;
mod movie_player;
mod player_handlers;
mod search;
mod settings;
mod streaming;
mod tmdb;
mod video;

use std::sync::Arc;
use tokio::sync::Mutex;

use iced::widget::container;
use iced::{Element, Font, Length, Size, Subscription, Task, Theme};

use media::{
    ContentSection, DetailPopupData, Episode, Genre, HeaderState, ImageCache, LoadingState,
    MediaId, MediaItem, Message, Page, SearchFilters, BACKGROUND_BLACK,
};
use movie_player::{MoviePlayer, PlaybackProgressStore};
use settings::{AppSettings, SetupPage};
use tmdb::{load_genres, load_hero_content, load_initial_content, TmdbClient};
use video::{TrailerManager, VideoPlayer};

pub struct Movix {
    pub setup_page: Option<SetupPage>,
    pub current_page: Page,
    pub header_state: HeaderState,
    pub hero_content: Option<MediaItem>,
    pub content_sections: Vec<ContentSection>,
    pub search_query: String,
    pub search_results: Vec<MediaItem>,
    pub profile_menu_open: bool,
    pub loading_state: LoadingState,
    pub error_message: Option<String>,
    pub image_cache: ImageCache,
    pub hovered_card: Option<MediaId>,
    pub pending_hover_card: Option<MediaId>,
    pub hovered_section: Option<usize>,
    pub section_scroll_offsets: Vec<f32>,
    pub section_scroll_targets: Vec<f32>,
    pub tmdb_client: Option<TmdbClient>,
    pub trailer_manager: TrailerManager,
    pub hero_player: Arc<Mutex<VideoPlayer>>,
    pub card_player: Arc<Mutex<VideoPlayer>>,
    pub trailer_cache: std::collections::HashMap<MediaId, Option<String>>,
    pub stream_url_cache: std::collections::HashMap<MediaId, String>,
    pub hero_visible: bool,
    pub main_scroll_offset: f32,
    pub hero_video_frame: Option<iced::widget::image::Handle>,
    pub card_video_frame: Option<iced::widget::image::Handle>,
    pub hero_muted: bool,
    pub hero_ended: bool,
    pub movie_player: Arc<Mutex<MoviePlayer>>,
    pub movie_player_active: bool,
    pub movie_player_media_id: Option<MediaId>,
    pub movie_player_title: Option<String>,
    pub movie_player_frame: Option<iced::widget::image::Handle>,
    pub movie_player_controls_visible: bool,
    pub movie_player_controls_timer: Option<std::time::Instant>,
    pub movie_player_loading: bool,
    pub movie_player_position: f64,
    pub movie_player_duration: f64,
    pub movie_player_volume: f64,
    pub movie_player_muted: bool,
    pub movie_player_playing: bool,
    pub movie_player_error: Option<String>,
    pub progress_store: Arc<Mutex<PlaybackProgressStore>>,
    pub detail_popup_open: bool,
    pub detail_popup_media_id: Option<MediaId>,
    pub detail_popup_data: Option<DetailPopupData>,
    pub detail_selected_season: Option<u32>,
    pub detail_episodes: Vec<Episode>,
    pub detail_hovered_card: Option<MediaId>,
    pub pending_detail_hover_card: Option<MediaId>,
    pub detail_player: Arc<Mutex<VideoPlayer>>,
    pub detail_video_frame: Option<iced::widget::image::Handle>,
    pub search_active: bool,
    pub search_filters: SearchFilters,
    pub filtered_results: Vec<MediaItem>,
    pub genre_list: Vec<Genre>,
    pub search_debounce_timer: Option<std::time::Instant>,
}

impl Default for Movix {
    fn default() -> Self {
        let progress_store = Arc::new(Mutex::new(PlaybackProgressStore::new()));
        Self {
            setup_page: None,
            current_page: Page::Home,
            header_state: HeaderState::default(),
            hero_content: None,
            content_sections: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            profile_menu_open: false,
            loading_state: LoadingState::Loading,
            error_message: None,
            image_cache: ImageCache::new(),
            hovered_card: None,
            pending_hover_card: None,
            hovered_section: None,
            section_scroll_offsets: Vec::new(),
            section_scroll_targets: Vec::new(),
            tmdb_client: None,
            trailer_manager: TrailerManager::new(),
            hero_player: Arc::new(Mutex::new(
                VideoPlayer::new().expect("Failed to init hero player"),
            )),
            card_player: Arc::new(Mutex::new(
                VideoPlayer::new().expect("Failed to init card player"),
            )),
            trailer_cache: std::collections::HashMap::new(),
            stream_url_cache: std::collections::HashMap::new(),
            hero_visible: true,
            main_scroll_offset: 0.0,
            hero_video_frame: None,
            card_video_frame: None,
            hero_muted: false,
            hero_ended: false,
            movie_player: Arc::new(Mutex::new(
                MoviePlayer::new(progress_store.clone()).expect("Failed to init movie player"),
            )),
            movie_player_active: false,
            movie_player_media_id: None,
            movie_player_title: None,
            movie_player_frame: None,
            movie_player_controls_visible: true,
            movie_player_controls_timer: None,
            movie_player_loading: false,
            movie_player_position: 0.0,
            movie_player_duration: 0.0,
            movie_player_volume: 1.0,
            movie_player_muted: false,
            movie_player_playing: false,
            movie_player_error: None,
            progress_store,
            detail_popup_open: false,
            detail_popup_media_id: None,
            detail_popup_data: None,
            detail_selected_season: None,
            detail_episodes: Vec::new(),
            detail_hovered_card: None,
            pending_detail_hover_card: None,
            detail_player: Arc::new(Mutex::new(
                VideoPlayer::new().expect("Failed to init detail player"),
            )),
            detail_video_frame: None,
            search_active: false,
            search_filters: SearchFilters::default(),
            filtered_results: Vec::new(),
            genre_list: Vec::new(),
            search_debounce_timer: None,
        }
    }
}

impl Movix {
    fn new() -> (Self, Task<Message>) {
        let settings = match AppSettings::load() {
            Some(s) if s.is_valid() => s,
            _ => {
                return (
                    Self {
                        setup_page: Some(SetupPage::default()),
                        ..Default::default()
                    },
                    Task::none(),
                );
            }
        };

        let client = TmdbClient::from_settings(&settings);
        let content_client = client.clone();
        let hero_client = client.clone();
        let genres_client = client.clone();
        let load_content =
            Task::perform(load_initial_content(content_client), Message::ContentLoaded);
        let load_hero = Task::perform(load_hero_content(hero_client), Message::HeroLoaded);
        let load_genres = Task::perform(load_genres(genres_client), Message::GenresLoaded);

        (
            Self {
                tmdb_client: Some(client),
                ..Default::default()
            },
            Task::batch([load_content, load_hero, load_genres]),
        )
    }

    fn initialize_with_settings(&mut self, settings: AppSettings) -> Task<Message> {
        let client = TmdbClient::from_settings(&settings);
        self.tmdb_client = Some(client.clone());
        self.setup_page = None;
        self.loading_state = LoadingState::Loading;

        let content_client = client.clone();
        let hero_client = client.clone();
        let genres_client = client;

        Task::batch([
            Task::perform(load_initial_content(content_client), Message::ContentLoaded),
            Task::perform(load_hero_content(hero_client), Message::HeroLoaded),
            Task::perform(load_genres(genres_client), Message::GenresLoaded),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Setup(setup_msg) = message {
            if let Some(ref mut setup) = self.setup_page {
                if let Some(settings) = setup.update(setup_msg) {
                    return self.initialize_with_settings(settings);
                }
            }
            return Task::none();
        }
        handlers::handle_message(self, message)
    }

    fn view(&self) -> Element<'_, Message> {
        if let Some(ref setup) = self.setup_page {
            return setup.view().map(Message::Setup);
        }

        if self.movie_player_active {
            return container(self.view_movie_player_overlay())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(BACKGROUND_BLACK)),
                    ..Default::default()
                })
                .into();
        }

        let main_content = container(self.view_main_content())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(BACKGROUND_BLACK)),
                ..Default::default()
            });

        if self.detail_popup_open {
            let popup_overlay = self.view_detail_popup_overlay();
            return iced::widget::stack![main_content, popup_overlay]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        main_content.into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        let hero_playing = self
            .hero_player
            .try_lock()
            .map(|p: tokio::sync::MutexGuard<'_, VideoPlayer>| p.is_playing())
            .unwrap_or(false);
        let card_playing = self
            .card_player
            .try_lock()
            .map(|p: tokio::sync::MutexGuard<'_, VideoPlayer>| p.is_playing())
            .unwrap_or(false);
        let detail_playing = self
            .detail_player
            .try_lock()
            .map(|p: tokio::sync::MutexGuard<'_, VideoPlayer>| p.is_playing())
            .unwrap_or(false);
        let movie_playing = self.movie_player_active
            && self
                .movie_player
                .try_lock()
                .map(|p| p.has_pipeline())
                .unwrap_or(false);

        let mut subs = Vec::new();
        if hero_playing && !self.movie_player_active && !self.detail_popup_open {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(33))
                    .map(|_| Message::HeroFrameTick),
            );
        }
        if card_playing && !self.movie_player_active && !self.detail_popup_open {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(33))
                    .map(|_| Message::CardFrameTick),
            );
        }
        if detail_playing && self.detail_popup_open && !self.movie_player_active {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(33))
                    .map(|_| Message::DetailFrameTick),
            );
        }
        if movie_playing {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(16))
                    .map(|_| Message::MoviePlayerFrameTick),
            );
        }
        if let Some(timer) = self.search_debounce_timer {
            if timer.elapsed() >= std::time::Duration::from_millis(300) {
                subs.push(
                    iced::time::every(std::time::Duration::from_millis(50))
                        .map(|_| Message::SearchDebounceTriggered),
                );
            } else {
                subs.push(
                    iced::time::every(std::time::Duration::from_millis(50))
                        .map(|_| Message::HeroFrameTick),
                );
            }
        }
        Subscription::batch(subs)
    }
}

fn setup_gstreamer_paths() {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let plugin_path = exe_dir.join("lib").join("gstreamer-1.0");
            if plugin_path.exists() {
                std::env::set_var("GST_PLUGIN_PATH", &plugin_path);
            }
        }
    }
}

fn main() -> iced::Result {
    setup_gstreamer_paths();

    iced::application(Movix::new, Movix::update, Movix::view)
        .title("Movix")
        .theme(Movix::theme)
        .window_size(Size::new(1280.0, 720.0))
        .font(iced_fonts::BOOTSTRAP_FONT_BYTES)
        .default_font(Font::DEFAULT)
        .subscription(Movix::subscription)
        .run()
}
