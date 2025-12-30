use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use iced::Task;
use rodio::Sink;
use serde::Deserialize;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::media::{ContentSection, MediaId, MediaType, Message};
use crate::tmdb::ImageSize;
use crate::Movix;

fn get_ytdlp_path() -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(windows)]
            let ytdlp_name = "yt-dlp.exe";
            #[cfg(not(windows))]
            let ytdlp_name = "yt-dlp";
            let bundled_path = exe_dir.join(ytdlp_name);
            if bundled_path.exists() {
                return bundled_path.to_string_lossy().to_string();
            }
        }
    }
    let compile_time_path = env!("YTDLP_PATH");
    if std::path::Path::new(compile_time_path).exists() {
        return compile_time_path.to_string();
    }
    #[cfg(windows)]
    return "yt-dlp.exe".to_string();
    #[cfg(not(windows))]
    return "yt-dlp".to_string();
}

#[derive(Clone)]
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

enum PlayerCommand {
    Pause,
    Resume,
    ToggleMute,
    Shutdown,
}

pub struct VideoPlayer {
    current_media_id: Option<MediaId>,
    current_frame: Option<FrameData>,
    frame_receiver: Option<crossbeam_channel::Receiver<FrameData>>,
    command_sender: Option<crossbeam_channel::Sender<PlayerCommand>>,
    decoder_thread: Option<thread::JoinHandle<()>>,
    is_playing: bool,
    is_muted: Arc<AtomicBool>,
    is_ended: Arc<AtomicBool>,
    current_url: Option<String>,
    target_width: u32,
    target_height: u32,
}

impl VideoPlayer {
    pub fn new() -> Result<Self, String> {
        ffmpeg_next::init().map_err(|e| format!("FFmpeg init failed: {}", e))?;
        Ok(Self {
            current_media_id: None,
            current_frame: None,
            frame_receiver: None,
            command_sender: None,
            decoder_thread: None,
            is_playing: false,
            is_muted: Arc::new(AtomicBool::new(false)),
            is_ended: Arc::new(AtomicBool::new(false)),
            current_url: None,
            target_width: 640,
            target_height: 360,
        })
    }

    pub fn play(&mut self, media_id: MediaId, url: &str) -> Result<(), String> {
        self.stop();
        let (frame_tx, frame_rx) = crossbeam_channel::bounded(4);
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let url_clone = url.to_string();
        let width = self.target_width;
        let height = self.target_height;
        let is_muted = self.is_muted.clone();
        let is_ended = self.is_ended.clone();
        is_ended.store(false, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            run_decoder(
                url_clone, width, height, frame_tx, cmd_rx, is_muted, is_ended,
            );
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

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn has_pipeline(&self) -> bool {
        self.decoder_thread.is_some()
    }

    pub fn current_media_id(&self) -> Option<MediaId> {
        self.current_media_id
    }

    pub fn toggle_mute(&mut self) {
        let current = self.is_muted.load(Ordering::SeqCst);
        self.is_muted.store(!current, Ordering::SeqCst);
        if let Some(ref sender) = self.command_sender {
            let _ = sender.send(PlayerCommand::ToggleMute);
        }
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted.load(Ordering::SeqCst)
    }

    pub fn check_ended(&mut self) -> bool {
        if self.is_ended.load(Ordering::SeqCst) {
            self.is_playing = false;
            return true;
        }
        false
    }

    pub fn replay(&mut self) -> Result<(), String> {
        let media_id = self.current_media_id.ok_or("No media to replay")?;
        let url = self.current_url.clone().ok_or("No URL to replay")?;
        self.play(media_id, &url)
    }

    pub fn get_frame(&self) -> Option<FrameData> {
        self.current_frame.clone()
    }

    pub fn render_frame(&mut self) -> Option<FrameData> {
        let receiver = self.frame_receiver.as_ref()?;
        if let Ok(frame) = receiver.try_recv() {
            self.current_frame = Some(frame.clone());
            return Some(frame);
        }
        self.current_frame.clone()
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_decoder(
    url: String,
    target_width: u32,
    target_height: u32,
    frame_sender: crossbeam_channel::Sender<FrameData>,
    command_receiver: crossbeam_channel::Receiver<PlayerCommand>,
    is_muted: Arc<AtomicBool>,
    is_ended: Arc<AtomicBool>,
) {
    let (_stream, sink) = match create_audio_output() {
        Some(s) => s,
        None => {
            is_ended.store(true, Ordering::SeqCst);
            return;
        }
    };

    let mut ictx = match ffmpeg_next::format::input(&url) {
        Ok(ctx) => ctx,
        Err(_) => {
            is_ended.store(true, Ordering::SeqCst);
            return;
        }
    };

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
                PlayerCommand::ToggleMute => {
                    sink.set_volume(if is_muted.load(Ordering::SeqCst) {
                        0.0
                    } else {
                        1.0
                    });
                }
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
                        if let Some(ref mut sc) = scaler {
                            let mut rgb_frame = ffmpeg_next::frame::Video::empty();
                            if sc.run(&decoded, &mut rgb_frame).is_ok() {
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
                                    data: rgb_frame.data(0).to_vec(),
                                };
                                if frame_sender.send(frame).is_err() {
                                    is_ended.store(true, Ordering::SeqCst);
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
    is_ended.store(true, Ordering::SeqCst);
}

fn create_audio_output() -> Option<(std::mem::ManuallyDrop<rodio::OutputStream>, Sink)> {
    let stream = rodio::OutputStreamBuilder::open_default_stream().ok()?;
    let sink = Sink::connect_new(stream.mixer());
    Some((std::mem::ManuallyDrop::new(stream), sink))
}

#[derive(Clone)]
pub struct TrailerManager {
    url_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl TrailerManager {
    pub fn new() -> Self {
        Self {
            url_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_stream_url(&self, youtube_id: &str) -> Result<String, String> {
        {
            let cache = self.url_cache.read().await;
            if let Some(url) = cache.get(youtube_id) {
                return Ok(url.clone());
            }
        }
        let url = self.fetch_stream_url(youtube_id).await?;
        self.url_cache
            .write()
            .await
            .insert(youtube_id.to_string(), url.clone());
        Ok(url)
    }

    async fn fetch_stream_url(&self, youtube_id: &str) -> Result<String, String> {
        let video_url = format!("https://www.youtube.com/watch?v={}", youtube_id);
        let ytdlp_path = get_ytdlp_path();
        let mut cmd = Command::new(&ytdlp_path);
        cmd.args([
            "-f",
            "22/18/best[height<=720][ext=mp4]/best[height<=720]/best",
            "-g",
            "--no-playlist",
            "--no-warnings",
            "--no-check-certificates",
            &video_url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = tokio::time::timeout(std::time::Duration::from_secs(8), cmd.output())
            .await
            .map_err(|_| "Timeout")?
            .map_err(|e| e.to_string())?;

        let url = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if url.is_empty() {
            return Err("No URL returned".to_string());
        }
        Ok(url)
    }
}

pub fn select_best_trailer(videos: &[TrailerVideo]) -> Option<&TrailerVideo> {
    videos
        .iter()
        .filter(|v| v.site == "YouTube")
        .find(|v| v.video_type == "Trailer" && v.official)
        .or_else(|| {
            videos
                .iter()
                .find(|v| v.site == "YouTube" && v.video_type == "Trailer")
        })
        .or_else(|| {
            videos
                .iter()
                .find(|v| v.site == "YouTube" && v.video_type == "Teaser")
        })
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrailerVideo {
    pub key: String,
    pub site: String,
    #[serde(rename = "type")]
    pub video_type: String,
    #[serde(default)]
    pub official: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideosResponse {
    pub results: Vec<TrailerVideo>,
}

impl Movix {
    pub fn load_content_images(&self, sections: &[ContentSection]) -> Task<Message> {
        let Some(client) = &self.tmdb_client else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        for section in sections {
            for item in section.items.iter().take(10) {
                if let Some(poster_path) = &item.poster_path {
                    let url = client.image_url(poster_path, ImageSize::Poster);
                    if self.image_cache.get(&url).is_none() {
                        tasks.push(Task::done(Message::LoadImage(url)));
                    }
                }
            }
        }
        Task::batch(tasks)
    }

    pub fn load_hero_images(&self, item: &crate::media::MediaItem) -> Task<Message> {
        let Some(client) = &self.tmdb_client else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        if let Some(backdrop_path) = &item.backdrop_path {
            let url = client.image_url(backdrop_path, ImageSize::Backdrop);
            if self.image_cache.get(&url).is_none() {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
        if let Some(logo_path) = &item.logo_path {
            let url = client.image_url(logo_path, ImageSize::Original);
            if self.image_cache.get(&url).is_none() {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
        Task::batch(tasks)
    }

    pub fn load_hover_card_images(&self, media_id: MediaId) -> Task<Message> {
        let Some(client) = &self.tmdb_client else {
            return Task::none();
        };
        let item = self
            .content_sections
            .iter()
            .flat_map(|s| &s.items)
            .find(|i| i.id == media_id)
            .or_else(|| self.search_results.iter().find(|i| i.id == media_id));
        let Some(item) = item else {
            return Task::none();
        };

        let mut tasks = Vec::new();
        if let Some(backdrop_path) = &item.backdrop_path {
            let url = client.image_url(backdrop_path, ImageSize::Backdrop);
            if self.image_cache.get(&url).is_none() && !self.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }

        if item.logo_path.is_none() {
            let fetch_client = client.clone();
            let media_type = item.media_type.clone();
            tasks.push(Task::perform(
                async move { fetch_client.fetch_media_images(media_id, &media_type).await },
                move |result| Message::LogoLoaded(media_id, result),
            ));
        } else if let Some(logo_path) = &item.logo_path {
            let url = client.image_url(logo_path, ImageSize::Original);
            if self.image_cache.get(&url).is_none() && !self.image_cache.is_pending(&url) {
                tasks.push(Task::done(Message::LoadImage(url)));
            }
        }
        Task::batch(tasks)
    }

    pub fn load_visible_images(&self, section_index: usize, scroll_offset: f32) -> Task<Message> {
        let Some(client) = &self.tmdb_client else {
            return Task::none();
        };
        let Some(section) = self.content_sections.get(section_index) else {
            return Task::none();
        };

        let card_width = 162.0;
        let visible_width = 1200.0;
        let start_index = (scroll_offset / card_width).floor() as usize;
        let visible_count = (visible_width / card_width).ceil() as usize + 2;
        let end_index = (start_index + visible_count).min(section.items.len());

        let mut tasks = Vec::new();
        for item in section
            .items
            .iter()
            .skip(start_index)
            .take(end_index - start_index)
        {
            if let Some(poster_path) = &item.poster_path {
                let url = client.image_url(poster_path, ImageSize::Poster);
                if self.image_cache.get(&url).is_none() && !self.image_cache.is_pending(&url) {
                    tasks.push(Task::done(Message::LoadImage(url)));
                }
            }
        }
        Task::batch(tasks)
    }

    pub fn load_trailer_for_media(
        &self,
        media_id: MediaId,
        media_type: &MediaType,
    ) -> Task<Message> {
        if self.trailer_cache.contains_key(&media_id) {
            return Task::none();
        }
        let Some(client) = &self.tmdb_client else {
            return Task::none();
        };
        let fetch_client = client.clone();
        let mt = media_type.clone();
        Task::perform(
            async move { fetch_client.fetch_videos(media_id, &mt).await },
            move |result| Message::TrailerVideosLoaded(media_id, result),
        )
    }

    pub fn fetch_trailer_stream_url(&self, media_id: MediaId, youtube_id: String) -> Task<Message> {
        let manager = self.trailer_manager.clone();
        Task::perform(
            async move { manager.get_stream_url(&youtube_id).await },
            move |result| Message::TrailerStreamUrlLoaded(media_id, result),
        )
    }

    pub fn load_trailer_for_hovered_card(&self, media_id: MediaId) -> Task<Message> {
        let pause_hero = Task::done(Message::PauseHeroTrailer);

        if self.stream_url_cache.contains_key(&media_id) {
            return Task::batch([pause_hero, Task::done(Message::PlayCardTrailer(media_id))]);
        }

        if let Some(cached) = self.trailer_cache.get(&media_id) {
            if let Some(youtube_id) = cached {
                let fetch_task = self.fetch_trailer_stream_url(media_id, youtube_id.clone());
                return Task::batch([pause_hero, fetch_task]);
            }
            return Task::none();
        }

        let item = self
            .content_sections
            .iter()
            .flat_map(|s| &s.items)
            .find(|i| i.id == media_id)
            .or_else(|| self.search_results.iter().find(|i| i.id == media_id));
        let Some(item) = item else {
            return Task::none();
        };
        let load_task = self.load_trailer_for_media(media_id, &item.media_type);
        Task::batch([pause_hero, load_task])
    }

    pub fn preload_trailer_urls(&self, sections: &[ContentSection]) -> Task<Message> {
        let Some(client) = &self.tmdb_client else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        for section in sections.iter().take(2) {
            for item in section.items.iter().take(5) {
                if self.trailer_cache.contains_key(&item.id) {
                    continue;
                }
                let fetch_client = client.clone();
                let media_id = item.id;
                let media_type = item.media_type.clone();
                tasks.push(Task::perform(
                    async move { fetch_client.fetch_videos(media_id, &media_type).await },
                    move |result| Message::TrailerVideosLoaded(media_id, result),
                ));
            }
        }
        Task::batch(tasks)
    }
}
