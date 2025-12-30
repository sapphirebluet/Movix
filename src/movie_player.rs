use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::sync::Mutex;

use iced::widget::{button, column, container, row, slider, text, Space};
use iced::{Border, Color, Element, Length, Padding, Shadow};
use rodio::Sink;

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

enum PlayerCommand {
    Pause,
    Resume,
    SetVolume(f32),
    Shutdown,
}

struct SharedState {
    position: AtomicU64,
    duration: AtomicU64,
    is_ended: AtomicBool,
}

impl SharedState {
    fn new() -> Self {
        Self {
            position: AtomicU64::new(0),
            duration: AtomicU64::new(0),
            is_ended: AtomicBool::new(false),
        }
    }
}

pub struct MoviePlayer {
    current_media_id: Option<MediaId>,
    current_frame: Option<FrameData>,
    frame_receiver: Option<crossbeam_channel::Receiver<FrameData>>,
    command_sender: Option<crossbeam_channel::Sender<PlayerCommand>>,
    decoder_thread: Option<thread::JoinHandle<()>>,
    shared_state: Arc<SharedState>,
    is_playing: bool,
    is_muted: bool,
    volume: f32,
    current_url: Option<String>,
    progress_store: Arc<Mutex<PlaybackProgressStore>>,
    target_width: u32,
    target_height: u32,
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
        ffmpeg_next::init().map_err(|e| format!("FFmpeg init failed: {}", e))?;
        Ok(Self {
            current_media_id: None,
            current_frame: None,
            frame_receiver: None,
            command_sender: None,
            decoder_thread: None,
            shared_state: Arc::new(SharedState::new()),
            is_playing: false,
            is_muted: false,
            volume: 1.0,
            current_url: None,
            progress_store,
            target_width: 1920,
            target_height: 1080,
        })
    }

    pub fn play(&mut self, media_id: MediaId, url: &str) -> Result<(), String> {
        self.stop();
        let (frame_tx, frame_rx) = crossbeam_channel::bounded(4);
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let url_clone = url.to_string();
        let width = self.target_width;
        let height = self.target_height;
        let shared = Arc::new(SharedState::new());
        self.shared_state = shared.clone();

        let handle = thread::spawn(move || {
            run_movie_decoder(url_clone, width, height, frame_tx, cmd_rx, shared);
        });

        self.frame_receiver = Some(frame_rx);
        self.command_sender = Some(cmd_tx);
        self.decoder_thread = Some(handle);
        self.current_media_id = Some(media_id);
        self.current_url = Some(url.to_string());
        self.is_playing = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sender) = self.command_sender.take() {
            let _ = sender.send(PlayerCommand::Shutdown);
        }
        if let Some(handle) = self.decoder_thread.take() {
            let _ = handle.join();
        }
        self.frame_receiver = None;
        self.current_media_id = None;
        self.current_url = None;
        self.is_playing = false;
        self.current_frame = None;
    }

    pub fn pause(&mut self) {
        if let Some(ref sender) = self.command_sender {
            let _ = sender.send(PlayerCommand::Pause);
        }
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        if let Some(ref sender) = self.command_sender {
            let _ = sender.send(PlayerCommand::Resume);
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
        self.decoder_thread.is_some()
    }

    pub fn current_media_id(&self) -> Option<MediaId> {
        self.current_media_id
    }

    pub fn set_volume(&mut self, v: f64) {
        self.volume = v.clamp(0.0, 1.0) as f32;
        if let Some(ref sender) = self.command_sender {
            let vol = if self.is_muted { 0.0 } else { self.volume };
            let _ = sender.send(PlayerCommand::SetVolume(vol));
        }
    }

    pub fn volume(&self) -> f64 {
        self.volume as f64
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
        if let Some(ref sender) = self.command_sender {
            let vol = if self.is_muted { 0.0 } else { self.volume };
            let _ = sender.send(PlayerCommand::SetVolume(vol));
        }
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn seek(&mut self, _pos: f64) {}
    pub fn seek_relative(&mut self, _delta: f64) {}

    pub fn position(&self) -> f64 {
        f64::from_bits(self.shared_state.position.load(Ordering::SeqCst))
    }

    pub fn duration(&self) -> f64 {
        f64::from_bits(self.shared_state.duration.load(Ordering::SeqCst))
    }

    pub fn check_ended(&self) -> bool {
        self.shared_state.is_ended.load(Ordering::SeqCst)
    }

    pub fn get_new_frame(&mut self) -> Option<FrameData> {
        let receiver = self.frame_receiver.as_ref()?;
        if let Ok(frame) = receiver.try_recv() {
            self.current_frame = Some(FrameData {
                width: frame.width,
                height: frame.height,
                data: frame.data.clone(),
            });
            return Some(frame);
        }
        None
    }

    pub fn get_current_frame(&self) -> Option<FrameData> {
        self.current_frame.as_ref().map(|f| FrameData {
            width: f.width,
            height: f.height,
            data: f.data.clone(),
        })
    }

    pub fn save_progress_sync(&self) {
        if let Some(id) = self.current_media_id {
            let pos = self.position();
            if pos > 5.0 {
                if let Ok(mut store) = self.progress_store.try_lock() {
                    store.set(id, pos);
                }
            }
        }
    }

    pub fn get_stored_position(&self, media_id: MediaId) -> Option<f64> {
        self.progress_store.try_lock().ok()?.get(media_id)
    }
}

impl Drop for MoviePlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_movie_decoder(
    url: String,
    target_width: u32,
    target_height: u32,
    frame_sender: crossbeam_channel::Sender<FrameData>,
    command_receiver: crossbeam_channel::Receiver<PlayerCommand>,
    shared_state: Arc<SharedState>,
) {
    let (_stream, sink) = match create_audio_output() {
        Some(s) => s,
        None => {
            shared_state.is_ended.store(true, Ordering::SeqCst);
            return;
        }
    };

    let mut ictx = match ffmpeg_next::format::input(&url) {
        Ok(ctx) => ctx,
        Err(_) => {
            shared_state.is_ended.store(true, Ordering::SeqCst);
            return;
        }
    };

    let duration_secs = ictx.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE);
    shared_state
        .duration
        .store(duration_secs.to_bits(), Ordering::SeqCst);

    let video_stream = ictx.streams().best(ffmpeg_next::media::Type::Video);
    let audio_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio);
    let video_index = video_stream.as_ref().map(|s| s.index());
    let audio_index = audio_stream.as_ref().map(|s| s.index());
    let video_time_base = video_stream.as_ref().map(|s| s.time_base());

    let mut video_decoder = video_stream.and_then(|s| {
        ffmpeg_next::codec::context::Context::from_parameters(s.parameters())
            .ok()?
            .decoder()
            .video()
            .ok()
    });

    let mut audio_decoder = audio_stream.and_then(|s| {
        ffmpeg_next::codec::context::Context::from_parameters(s.parameters())
            .ok()?
            .decoder()
            .audio()
            .ok()
    });

    let mut scaler = video_decoder.as_ref().and_then(|dec| {
        ffmpeg_next::software::scaling::Context::get(
            dec.format(),
            dec.width(),
            dec.height(),
            ffmpeg_next::format::Pixel::RGBA,
            target_width,
            target_height,
            ffmpeg_next::software::scaling::Flags::BILINEAR,
        )
        .ok()
    });

    let mut resampler = audio_decoder.as_ref().and_then(|dec| {
        ffmpeg_next::software::resampling::Context::get(
            dec.format(),
            dec.channel_layout(),
            dec.rate(),
            ffmpeg_next::format::Sample::I16(ffmpeg_next::format::sample::Type::Packed),
            ffmpeg_next::ChannelLayout::STEREO,
            44100,
        )
        .ok()
    });

    let playback_start = std::time::Instant::now();
    let mut pause_offset = std::time::Duration::ZERO;
    let mut pause_start: Option<std::time::Instant> = None;
    let mut is_paused = false;

    for (pkt_stream, packet) in ictx.packets() {
        while let Ok(cmd) = command_receiver.try_recv() {
            match cmd {
                PlayerCommand::Shutdown => return,
                PlayerCommand::Pause => {
                    is_paused = true;
                    pause_start = Some(std::time::Instant::now());
                    sink.pause();
                }
                PlayerCommand::Resume => {
                    is_paused = false;
                    if let Some(ps) = pause_start.take() {
                        pause_offset += ps.elapsed();
                    }
                    sink.play();
                }
                PlayerCommand::SetVolume(v) => sink.set_volume(v),
            }
        }

        if is_paused {
            thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }

        let stream_index = pkt_stream.index();

        if Some(stream_index) == audio_index {
            if let (Some(ref mut decoder), Some(ref mut resamp)) =
                (&mut audio_decoder, &mut resampler)
            {
                if decoder.send_packet(&packet).is_ok() {
                    let mut decoded = ffmpeg_next::frame::Audio::empty();
                    while decoder.receive_frame(&mut decoded).is_ok() {
                        let mut resampled = ffmpeg_next::frame::Audio::empty();
                        if resamp.run(&decoded, &mut resampled).is_ok() {
                            let data = resampled.data(0);
                            let samples: Vec<f32> = data
                                .chunks_exact(2)
                                .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                                .collect();
                            let source = rodio::buffer::SamplesBuffer::new(2, 44100, samples);
                            sink.append(source);
                        }
                    }
                }
            }
        }

        if Some(stream_index) == video_index {
            if let Some(ref mut decoder) = video_decoder {
                if decoder.send_packet(&packet).is_ok() {
                    let mut decoded = ffmpeg_next::frame::Video::empty();
                    while decoder.receive_frame(&mut decoded).is_ok() {
                        if let Some(tb) = video_time_base {
                            let pts = decoded.pts().unwrap_or(0);
                            let pos = pts as f64 * f64::from(tb);
                            shared_state.position.store(pos.to_bits(), Ordering::SeqCst);
                        }
                        if let Some(ref mut sc) = scaler {
                            let mut rgb = ffmpeg_next::frame::Video::empty();
                            if sc.run(&decoded, &mut rgb).is_ok() {
                                if let Some(tb) = video_time_base {
                                    let pts = decoded.pts().unwrap_or(0);
                                    let frame_time = std::time::Duration::from_secs_f64(
                                        pts as f64 * f64::from(tb),
                                    );
                                    let elapsed = playback_start.elapsed() - pause_offset;
                                    if frame_time > elapsed {
                                        thread::sleep(frame_time - elapsed);
                                    }
                                }
                                let frame = FrameData {
                                    width: target_width,
                                    height: target_height,
                                    data: rgb.data(0).to_vec(),
                                };
                                if frame_sender.send(frame).is_err() {
                                    shared_state.is_ended.store(true, Ordering::SeqCst);
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    sink.sleep_until_end();
    shared_state.is_ended.store(true, Ordering::SeqCst);
}

fn create_audio_output() -> Option<(std::mem::ManuallyDrop<rodio::OutputStream>, Sink)> {
    let stream = rodio::OutputStreamBuilder::open_default_stream().ok()?;
    let sink = Sink::connect_new(stream.mixer());
    Some((std::mem::ManuallyDrop::new(stream), sink))
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
