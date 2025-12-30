use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, RwLock as StdRwLock};

use gstreamer as gst;
use gstreamer::prelude::*;
use iced::Task;
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

struct SharedFrame {
    frame: Option<FrameData>,
}

pub struct VideoPlayer {
    pipeline: Option<gst::Pipeline>,
    current_media_id: Option<MediaId>,
    shared_frame: Arc<StdRwLock<SharedFrame>>,
    is_playing: bool,
    is_muted: bool,
    is_ended: bool,
    current_url: Option<String>,
}

impl VideoPlayer {
    pub fn new() -> Result<Self, String> {
        gst::init().map_err(|e| format!("Failed to init GStreamer: {}", e))?;
        Ok(Self {
            pipeline: None,
            current_media_id: None,
            shared_frame: Arc::new(StdRwLock::new(SharedFrame { frame: None })),
            is_playing: false,
            is_muted: false,
            is_ended: false,
            current_url: None,
        })
    }

    pub fn play(&mut self, media_id: MediaId, url: &str) -> Result<(), String> {
        self.stop();
        let pipeline = gst::Pipeline::new();
        let playbin = gst::ElementFactory::make("playbin3")
            .property("uri", url)
            .property("buffer-size", 2 * 1024 * 1024i32)
            .property("buffer-duration", 3_000_000_000i64)
            .build()
            .map_err(|e| format!("Failed to create playbin3: {}", e))?;

        let video_sink = self.create_video_sink()?;
        let audio_sink = gst::ElementFactory::make("autoaudiosink")
            .build()
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        playbin.set_property("video-sink", &video_sink);
        playbin.set_property("audio-sink", &audio_sink);
        playbin.set_property("mute", self.is_muted);

        pipeline
            .add(&playbin)
            .map_err(|e| format!("Failed to add playbin: {}", e))?;
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| format!("Failed to start: {:?}", e))?;

        self.pipeline = Some(pipeline);
        self.current_media_id = Some(media_id);
        self.current_url = Some(url.to_string());
        self.is_playing = true;
        self.is_ended = false;
        Ok(())
    }

    fn create_video_sink(&self) -> Result<gst::Element, String> {
        let bin = gst::Bin::new();
        let queue = gst::ElementFactory::make("queue")
            .property("max-size-buffers", 4u32)
            .property("max-size-time", 0u64)
            .property("max-size-bytes", 0u32)
            .build()
            .map_err(|e| format!("queue: {}", e))?;

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .map_err(|e| format!("Failed to create videoconvert: {}", e))?;

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("format", "RGBA")
                    .build(),
            )
            .build()
            .map_err(|e| format!("Failed to create capsfilter: {}", e))?;

        let appsink = gst::ElementFactory::make("appsink")
            .property("emit-signals", true)
            .property("sync", true)
            .property("max-buffers", 2u32)
            .property("drop", false)
            .build()
            .map_err(|e| format!("Failed to create appsink: {}", e))?;

        bin.add_many([&queue, &videoconvert, &capsfilter, &appsink])
            .map_err(|e| format!("Failed to add elements: {}", e))?;
        gst::Element::link_many([&queue, &videoconvert, &capsfilter, &appsink])
            .map_err(|e| format!("Failed to link elements: {}", e))?;

        let pad = queue.static_pad("sink").ok_or("Failed to get sink pad")?;
        let ghost_pad =
            gst::GhostPad::with_target(&pad).map_err(|e| format!("Ghost pad error: {}", e))?;
        bin.add_pad(&ghost_pad)
            .map_err(|e| format!("Failed to add ghost pad: {}", e))?;

        let appsink = appsink
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| "Failed to downcast to AppSink")?;
        let shared = self.shared_frame.clone();

        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;
                    let info = gstreamer_video::VideoInfo::from_caps(caps)
                        .map_err(|_| gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                    if let Ok(mut guard) = shared.write() {
                        guard.frame = Some(FrameData {
                            width: info.width(),
                            height: info.height(),
                            data: map.as_slice().to_vec(),
                        });
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
        Ok(bin.upcast())
    }

    pub fn stop(&mut self) {
        if let Some(pipeline) = self.pipeline.take() {
            let _ = pipeline.set_state(gst::State::Null);
        }
        self.current_media_id = None;
        self.current_url = None;
        self.is_playing = false;
        self.is_ended = false;
        if let Ok(mut guard) = self.shared_frame.write() {
            guard.frame = None;
        }
    }

    pub fn pause(&mut self) {
        if let Some(ref pipeline) = self.pipeline {
            let _ = pipeline.set_state(gst::State::Paused);
            self.is_playing = false;
        }
    }

    pub fn resume(&mut self) {
        if let Some(ref pipeline) = self.pipeline {
            let _ = pipeline.set_state(gst::State::Playing);
            self.is_playing = true;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn has_pipeline(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn current_media_id(&self) -> Option<MediaId> {
        self.current_media_id
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
        if let Some(ref pipeline) = self.pipeline {
            for element in pipeline.iterate_elements().into_iter().flatten() {
                if element.name().as_str().contains("playbin") {
                    element.set_property("mute", self.is_muted);
                    break;
                }
            }
        }
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn check_ended(&mut self) -> bool {
        if let Some(ref pipeline) = self.pipeline {
            let bus = pipeline.bus().unwrap();
            while let Some(msg) = bus.pop() {
                if let gst::MessageView::Eos(_) = msg.view() {
                    self.is_ended = true;
                    self.is_playing = false;
                    return true;
                }
            }
        }
        false
    }

    pub fn replay(&mut self) -> Result<(), String> {
        let media_id = self.current_media_id.ok_or("No media to replay")?;
        let url = self.current_url.clone().ok_or("No URL to replay")?;
        self.play(media_id, &url)
    }

    pub fn get_frame(&self) -> Option<FrameData> {
        self.shared_frame.read().ok().and_then(|g| g.frame.clone())
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
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
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            Command::new(&ytdlp_path)
                .args([
                    "-f",
                    "22/18/best[height<=720][ext=mp4]/best[height<=720]/best",
                    "-g",
                    "--no-playlist",
                    "--no-warnings",
                    "--no-check-certificates",
                    &video_url,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output(),
        )
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

        if let Some(url) = self.stream_url_cache.get(&media_id) {
            let player = self.card_player.clone();
            let url = url.clone();
            let play_task = Task::perform(
                async move {
                    let mut p = player.lock().await;
                    let _ = p.play(media_id, &url);
                },
                |_| Message::CardFrameTick,
            );
            return Task::batch([pause_hero, play_task]);
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
