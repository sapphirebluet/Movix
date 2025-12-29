use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

use gstreamer as gst;
use gstreamer::prelude::*;
use iced::widget::{button, column, container, row, slider, text, Space};
use iced::{Border, Color, Element, Length, Padding, Shadow};

use crate::media::{MediaId, Message, NETFLIX_RED, TEXT_GRAY, TEXT_WHITE};
use crate::streaming;
use crate::Movix;

const ICON_ARROW_LEFT: char = '\u{F12F}';
const ICON_PLAY_FILL: char = '\u{F4F4}';
const ICON_PAUSE_FILL: char = '\u{F4C3}';
const ICON_SKIP_BACKWARD_FILL: char = '\u{F552}';
const ICON_SKIP_FORWARD_FILL: char = '\u{F555}';
const ICON_VOLUME_UP_FILL: char = '\u{F611}';
const ICON_VOLUME_MUTE_FILL: char = '\u{F608}';
const ICON_FULLSCREEN: char = '\u{F31E}';

pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

struct SharedFrame {
    current: Option<FrameData>,
    pending: Option<FrameData>,
}

pub struct MoviePlayer {
    pipeline: Option<gst::Pipeline>,
    playbin: Option<gst::Element>,
    current_media_id: Option<MediaId>,
    shared_frame: Arc<RwLock<SharedFrame>>,
    is_playing: bool,
    is_muted: bool,
    volume: f64,
    current_url: Option<String>,
    progress_store: Arc<Mutex<PlaybackProgressStore>>,
}

#[derive(Clone, Default)]
pub struct PlaybackProgressStore {
    progress: HashMap<MediaId, f64>,
    storage_path: Option<PathBuf>,
}

impl PlaybackProgressStore {
    pub fn new() -> Self {
        let storage_path = std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".local/share/movix/playback_progress.json"));

        if let Some(ref path) = storage_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut store = Self {
            progress: HashMap::new(),
            storage_path,
        };
        store.load();
        store
    }

    fn load(&mut self) {
        let Some(ref path) = self.storage_path else {
            return;
        };
        if !path.exists() {
            return;
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str(&content) {
                self.progress = data;
            }
        }
    }

    fn save(&self) {
        let Some(ref path) = self.storage_path else {
            return;
        };
        if let Ok(json) = serde_json::to_string(&self.progress) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn get(&self, media_id: MediaId) -> Option<f64> {
        self.progress.get(&media_id).copied()
    }

    pub fn set(&mut self, media_id: MediaId, position: f64) {
        self.progress.insert(media_id, position);
        self.save();
    }
}

impl MoviePlayer {
    pub fn new(progress_store: Arc<Mutex<PlaybackProgressStore>>) -> Result<Self, String> {
        gst::init().map_err(|e| format!("Failed to init GStreamer: {}", e))?;
        Ok(Self {
            pipeline: None,
            playbin: None,
            current_media_id: None,
            shared_frame: Arc::new(RwLock::new(SharedFrame {
                current: None,
                pending: None,
            })),
            is_playing: false,
            is_muted: false,
            volume: 1.0,
            current_url: None,
            progress_store,
        })
    }

    pub fn play(&mut self, media_id: MediaId, url: &str) -> Result<(), String> {
        self.stop();

        let pipeline = gst::Pipeline::new();

        let playbin = gst::ElementFactory::make("playbin3")
            .property("uri", url)
            .property("buffer-size", 4 * 1024 * 1024i32)
            .property("buffer-duration", 5_000_000_000i64)
            .build()
            .map_err(|e| format!("Failed to create playbin3: {}", e))?;

        let video_sink = self.create_video_sink()?;

        playbin.set_property("video-sink", &video_sink);
        playbin.set_property("volume", self.volume);
        playbin.set_property("mute", self.is_muted);

        pipeline
            .add(&playbin)
            .map_err(|e| format!("Failed to add: {}", e))?;
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| format!("Failed: {:?}", e))?;

        self.playbin = Some(playbin);
        self.pipeline = Some(pipeline);
        self.current_media_id = Some(media_id);
        self.current_url = Some(url.to_string());
        self.is_playing = true;
        Ok(())
    }

    fn create_video_sink(&self) -> Result<gst::Element, String> {
        let bin = gst::Bin::new();

        let queue = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 3u32)
            .property("max-size-time", 0u64)
            .property("max-size-bytes", 0u32)
            .build()
            .map_err(|e| format!("queue: {}", e))?;

        let convert = gst::ElementFactory::make("videoconvert")
            .build()
            .map_err(|e| format!("videoconvert: {}", e))?;

        let caps = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("format", "RGBA")
                    .build(),
            )
            .build()
            .map_err(|e| format!("capsfilter: {}", e))?;

        let sink = gst::ElementFactory::make("appsink")
            .property("emit-signals", true)
            .property("sync", true)
            .property("max-buffers", 1u32)
            .property("drop", true)
            .build()
            .map_err(|e| format!("appsink: {}", e))?;

        bin.add_many([&queue, &convert, &caps, &sink])
            .map_err(|e| format!("add: {}", e))?;
        gst::Element::link_many([&queue, &convert, &caps, &sink])
            .map_err(|e| format!("link: {}", e))?;

        let pad = queue.static_pad("sink").ok_or("no pad")?;
        bin.add_pad(&gst::GhostPad::with_target(&pad).map_err(|e| format!("ghost: {}", e))?)
            .map_err(|e| format!("add pad: {}", e))?;

        let appsink = sink
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| "downcast")?;

        let shared = self.shared_frame.clone();

        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |s| {
                    let sample = s.pull_sample().map_err(|_| gst::FlowError::Error)?;
                    let buf = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;
                    let info = gstreamer_video::VideoInfo::from_caps(caps)
                        .map_err(|_| gst::FlowError::Error)?;
                    let map = buf.map_readable().map_err(|_| gst::FlowError::Error)?;

                    let expected_size = (info.width() * info.height() * 4) as usize;
                    if map.len() < expected_size {
                        return Ok(gst::FlowSuccess::Ok);
                    }

                    let frame = FrameData {
                        width: info.width(),
                        height: info.height(),
                        data: map.as_slice().to_vec(),
                    };

                    if let Ok(mut guard) = shared.write() {
                        guard.pending = Some(frame);
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
        Ok(bin.upcast())
    }

    pub fn stop(&mut self) {
        if let Some(p) = self.pipeline.take() {
            let _ = p.set_state(gst::State::Null);
        }
        self.playbin = None;
        self.current_media_id = None;
        self.current_url = None;
        self.is_playing = false;
        if let Ok(mut guard) = self.shared_frame.write() {
            guard.current = None;
            guard.pending = None;
        }
    }

    pub fn pause(&mut self) {
        if let Some(ref p) = self.pipeline {
            let _ = p.set_state(gst::State::Paused);
        }
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        if let Some(ref p) = self.pipeline {
            let _ = p.set_state(gst::State::Playing);
        }
        self.is_playing = true;
    }

    pub fn toggle_play_pause(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.resume();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn has_pipeline(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn set_volume(&mut self, v: f64) {
        self.volume = v.clamp(0.0, 1.0);
        if let Some(ref pb) = self.playbin {
            pb.set_property("volume", self.volume);
        }
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
        if let Some(ref pb) = self.playbin {
            pb.set_property("mute", self.is_muted);
        }
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn seek(&mut self, pos: f64) {
        if let Some(ref p) = self.pipeline {
            let _ = p.seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_seconds_f64(pos),
            );
        }
    }

    pub fn seek_relative(&mut self, delta: f64) {
        self.seek((self.position() + delta).clamp(0.0, self.duration()));
    }

    pub fn position(&self) -> f64 {
        self.pipeline
            .as_ref()
            .and_then(|p| p.query_position::<gst::ClockTime>())
            .map(|t| t.seconds_f64())
            .unwrap_or(0.0)
    }

    pub fn duration(&self) -> f64 {
        self.pipeline
            .as_ref()
            .and_then(|p| p.query_duration::<gst::ClockTime>())
            .map(|t| t.seconds_f64())
            .unwrap_or(0.0)
    }

    pub fn check_ended(&self) -> bool {
        self.pipeline.as_ref().map_or(false, |p| {
            p.bus().map_or(false, |bus| {
                while let Some(msg) = bus.pop() {
                    if matches!(msg.view(), gst::MessageView::Eos(_)) {
                        return true;
                    }
                }
                false
            })
        })
    }

    pub fn get_new_frame(&mut self) -> Option<FrameData> {
        let mut guard = self.shared_frame.write().ok()?;
        if let Some(frame) = guard.pending.take() {
            guard.current = Some(FrameData {
                width: frame.width,
                height: frame.height,
                data: frame.data.clone(),
            });
            return Some(frame);
        }
        None
    }

    pub fn get_current_frame(&self) -> Option<FrameData> {
        let guard = self.shared_frame.read().ok()?;
        guard.current.as_ref().map(|f| FrameData {
            width: f.width,
            height: f.height,
            data: f.data.clone(),
        })
    }

    pub async fn save_progress(&self) {
        if let Some(id) = self.current_media_id {
            let pos = self.position();
            if pos > 5.0 {
                self.progress_store.lock().await.set(id, pos);
            }
        }
    }
}

impl Drop for MoviePlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct VoeStreamResolver;

impl VoeStreamResolver {
    pub async fn get_download_url(title: &str) -> Result<String, String> {
        streaming::create_default_service()
            .get_stream_url(title)
            .await
            .map_err(|e| e.to_string())
    }
}

pub fn format_time(secs: f64) -> String {
    let t = secs as u64;
    let h = t / 3600;
    let m = (t % 3600) / 60;
    let s = t % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

fn icon(codepoint: char) -> iced::widget::Text<'static> {
    text(codepoint.to_string()).font(iced::Font {
        family: iced::font::Family::Name("bootstrap-icons"),
        ..Default::default()
    })
}

impl Movix {
    pub fn view_movie_player_overlay(&self) -> Element<'_, Message> {
        let video = self.view_movie_video();
        let controls = self.view_movie_controls_overlay();

        iced::widget::stack![video, controls]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_movie_video(&self) -> Element<'_, Message> {
        if let Some(ref err) = self.movie_player_error {
            return self.view_movie_error(err);
        }
        if self.movie_player_loading {
            return self.view_movie_loading();
        }
        match &self.movie_player_frame {
            Some(handle) => container(
                iced::widget::image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Contain),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into(),
            None => self.view_movie_loading(),
        }
    }

    fn view_movie_error(&self, err: &str) -> Element<'_, Message> {
        let title = self.movie_player_title.clone().unwrap_or_default();
        container(
            column![
                text("Failed to load").size(24).color(NETFLIX_RED),
                text(err.to_string()).size(14).color(TEXT_GRAY),
                text(title).size(16).color(TEXT_WHITE)
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        })
        .into()
    }

    fn view_movie_loading(&self) -> Element<'_, Message> {
        let title = self.movie_player_title.clone().unwrap_or_default();
        container(
            column![
                text("Loading...").size(24).color(TEXT_WHITE),
                text(title).size(16).color(TEXT_GRAY)
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        })
        .into()
    }

    fn view_movie_controls_overlay(&self) -> Element<'_, Message> {
        if !self.movie_player_controls_visible && !self.movie_player_loading {
            return container(Space::new().width(0).height(0))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        let back_btn = button(icon(ICON_ARROW_LEFT).size(24).color(TEXT_WHITE))
            .padding(Padding::new(12.0))
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0,
                    0.0,
                    0.0,
                    if matches!(status, button::Status::Hovered) {
                        0.5
                    } else {
                        0.3
                    },
                ))),
                text_color: TEXT_WHITE,
                border: Border {
                    radius: 24.0.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: false,
            })
            .on_press(Message::MoviePlayerClose);

        let top = container(back_btn)
            .width(Length::Fill)
            .padding(Padding::new(16.0));

        let bottom = self.view_movie_bottom_controls();

        column![
            top,
            Space::new().width(Length::Fill).height(Length::Fill),
            bottom
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_movie_bottom_controls(&self) -> Element<'_, Message> {
        let time_cur = format_time(self.movie_player_position);
        let time_tot = format_time(self.movie_player_duration);
        let max_dur = self.movie_player_duration.max(1.0);

        let slider_widget = slider(
            0.0..=max_dur,
            self.movie_player_position,
            Message::MoviePlayerSeek,
        )
        .width(Length::Fill)
        .height(4.0)
        .style(|_, _| slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    iced::Background::Color(NETFLIX_RED),
                    iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
                ),
                width: 4.0,
                border: Border::default(),
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: 6.0 },
                background: iced::Background::Color(TEXT_WHITE),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        });

        let progress_row = row![
            text(time_cur).size(12).color(TEXT_WHITE),
            slider_widget,
            text(time_tot).size(12).color(TEXT_WHITE)
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let play_icon = if self.movie_player_playing {
            ICON_PAUSE_FILL
        } else {
            ICON_PLAY_FILL
        };
        let vol_icon = if self.movie_player_muted {
            ICON_VOLUME_MUTE_FILL
        } else {
            ICON_VOLUME_UP_FILL
        };

        let vol_slider = slider(
            0.0..=1.0,
            self.movie_player_volume,
            Message::MoviePlayerSetVolume,
        )
        .width(Length::Fixed(80.0))
        .height(4.0)
        .style(|_, _| slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    iced::Background::Color(TEXT_WHITE),
                    iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
                ),
                width: 4.0,
                border: Border::default(),
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: 5.0 },
                background: iced::Background::Color(TEXT_WHITE),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        });

        let title = self.movie_player_title.clone().unwrap_or_default();

        let left = row![
            self.ctrl_btn(play_icon, Message::MoviePlayerTogglePlay),
            self.ctrl_btn(
                ICON_SKIP_BACKWARD_FILL,
                Message::MoviePlayerSeekRelative(-10.0)
            ),
            self.ctrl_btn(
                ICON_SKIP_FORWARD_FILL,
                Message::MoviePlayerSeekRelative(10.0)
            ),
            self.ctrl_btn(vol_icon, Message::MoviePlayerToggleMute),
            vol_slider
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        let center = container(text(title).size(14).color(TEXT_WHITE))
            .width(Length::Fill)
            .center_x(Length::Fill);

        let right = self.ctrl_btn(ICON_FULLSCREEN, Message::MoviePlayerToggleFullscreen);

        let controls_row = row![left, center, right]
            .align_y(iced::Alignment::Center)
            .width(Length::Fill);

        container(
            column![progress_row, controls_row]
                .spacing(8)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding(Padding::new(12.0).left(20.0).right(20.0))
        .style(|_| container::Style {
            background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(0.0)
                    .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.9))
                    .add_stop(0.6, Color::from_rgba(0.0, 0.0, 0.0, 0.4))
                    .add_stop(1.0, Color::TRANSPARENT),
            ))),
            ..Default::default()
        })
        .into()
    }

    fn ctrl_btn(&self, ic: char, msg: Message) -> Element<'_, Message> {
        button(icon(ic).size(18).color(TEXT_WHITE))
            .padding(Padding::new(8.0))
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    1.0,
                    1.0,
                    1.0,
                    if matches!(status, button::Status::Hovered) {
                        0.2
                    } else {
                        0.0
                    },
                ))),
                text_color: TEXT_WHITE,
                border: Border {
                    radius: 16.0.into(),
                    ..Default::default()
                },
                shadow: Shadow::default(),
                snap: false,
            })
            .on_press(msg)
            .into()
    }
}
