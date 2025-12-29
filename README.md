<picture>
  <img alt="Movix Preview" src="https://github.com/sapphirebluet/Movix/releases/download/assets/preview.png">
</picture>

<h2 align="center">Movix</h2>

<p align="center">
  A native desktop streaming client built with Rust and Iced, featuring a Netflix-inspired interface for browsing and playing movies and TV series with TMDB integration.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Iced-0.14-blue?style=for-the-badge" alt="Iced">
  <img src="https://img.shields.io/badge/GStreamer-Multimedia-green?style=for-the-badge&logo=gstreamer" alt="GStreamer">
  <img src="https://img.shields.io/badge/TMDB-API-01d277?style=for-the-badge&logo=themoviedatabase" alt="TMDB">
  <img src="https://img.shields.io/badge/License-GPL%20v3-blue?style=for-the-badge" alt="License">
</p>

<h3 align="center">Downloads</h3>

<p align="center">
  <a href="https://github.com/sapphirebluet/Movix/releases/latest/download/movix-windows-x64-setup.exe"><img src="https://img.shields.io/badge/Windows-Installer-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Windows Installer"></a>
  <a href="https://github.com/sapphirebluet/Movix/releases/latest/download/movix-windows-x64.exe"><img src="https://img.shields.io/badge/Windows-Portable-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Windows Portable"></a>
</p>

<p align="center">
  <a href="https://github.com/sapphirebluet/Movix/releases/latest/download/movix-x86_64.AppImage"><img src="https://img.shields.io/badge/Linux-AppImage-FCC624?style=for-the-badge&logo=linux&logoColor=black" alt="Linux AppImage"></a>
  <a href="https://github.com/sapphirebluet/Movix/releases/latest/download/movix-amd64.deb"><img src="https://img.shields.io/badge/Linux-.deb-FCC624?style=for-the-badge&logo=debian&logoColor=A81D33" alt="Linux .deb"></a>
  <a href="https://github.com/sapphirebluet/Movix/releases/latest/download/movix-x86_64.rpm"><img src="https://img.shields.io/badge/Linux-.rpm-FCC624?style=for-the-badge&logo=redhat&logoColor=EE0000" alt="Linux .rpm"></a>
  <a href="https://github.com/sapphirebluet/Movix/releases/latest/download/movix-x86_64.pkg.tar.zst"><img src="https://img.shields.io/badge/Linux-Arch-FCC624?style=for-the-badge&logo=archlinux&logoColor=1793D1" alt="Arch Linux"></a>
</p>

<h3 align="center">Mirrors</h3>

<p align="center">
  <a href="https://github.com/sapphirebluet/Movix">
    <img src="https://img.shields.io/badge/GitHub-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub Mirror">
  </a>
  <a href="https://codeberg.org/sapphirebluet/Movix">
    <img src="https://img.shields.io/badge/Codeberg-2185D0?style=for-the-badge&logo=codeberg&logoColor=white" alt="Codeberg Mirror">
  </a>
</p>

## Features

- Netflix-style UI with hero sections, content carousels, and expandable cards
- TMDB integration for movie/series metadata, posters, backdrops, and logos
- Video playback via GStreamer with trailer previews on hover
- Detail popups with cast, collections, seasons/episodes, and recommendations
- Search with filters (media type, genre, year range, rating, sort options)
- Playback progress persistence across sessions
- Image caching with color palette extraction for dynamic gradients
- Streaming provider architecture with pluggable resolvers

---

## Architecture

```
src/
├── main.rs              # Application entry, Iced setup, state management
├── settings.rs          # App settings persistence and setup page UI
├── components.rs        # Header, navigation, skeleton loading UI
├── cards.rs             # Content cards with hover expansion and video preview
├── hero.rs              # Hero section with backdrop video and metadata
├── search.rs            # Search page with filter panel and result grid
├── detail_popup.rs      # Modal detail view with mini-hero
├── detail_sections.rs   # Cast, seasons, collections, similar titles
├── detail_handlers.rs   # Detail popup state and data loading
├── handlers.rs          # Main message routing and event handling
├── player_handlers.rs   # Video player state management
├── movie_player.rs      # Full-screen movie player with controls
├── video.rs             # Trailer video player and YouTube URL resolution
├── media.rs             # Data types, color palette, image cache
├── tmdb.rs              # TMDB API client with response caching
└── streaming/
    ├── mod.rs           # StreamProvider and StreamResolver traits
    ├── providers/
    │   └── filmpalastto.rs  # Filmpalast.to stream page provider
    └── resolvers/
        └── voe.rs       # VOE.sx stream URL resolver with deobfuscation
```

---

## Core Components

### TMDB Client (`tmdb.rs`)

Handles all API communication with The Movie Database:

- Fetches trending, top-rated, and genre-filtered content
- Retrieves full media details including runtime, certification, and logos
- Loads cast, external IDs, keywords, and recommendations
- Implements response caching with configurable TTL
- Language configured via settings page

### Settings (`settings.rs`)

On first launch, displays a setup page to configure:

- TMDB API key (get free at themoviedb.org/settings/api)
- Language preference (e.g., en-US, de-DE, fr-FR)

Settings are persisted to `~/.config/movix/config.json` and loaded on subsequent launches.

### Video Playback (`video.rs`, `movie_player.rs`)

GStreamer-based video pipeline:

- `VideoPlayer`: Lightweight player for trailer previews with frame extraction
- `MoviePlayer`: Full-featured player with seek, volume, mute, and progress tracking
- `TrailerManager`: YouTube URL resolution via yt-dlp with caching
- Frame data extraction via `appsink` for Iced image rendering

### Streaming Service (`streaming/`)

Modular architecture for stream resolution:

```rust
pub trait StreamProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn get_stream_page_url(&self, title: &str) -> Result<String, StreamError>;
}

pub trait StreamResolver: Send + Sync {
    fn name(&self) -> &str;
    fn can_handle(&self, url: &str) -> bool;
    async fn resolve(&self, url: &str) -> Result<String, StreamError>;
}
```

The `VoeResolver` implements multi-layer deobfuscation:

1. ROT13 transformation
2. Custom marker stripping
3. Base64 decoding
4. Character shift
5. String reversal
6. Final Base64 decode

### Image Cache (`media.rs`)

- Disk-based caching in `~/.cache/movix/images/`
- Color palette extraction for dynamic UI gradients
- Pending request tracking to prevent duplicate fetches

### State Management (`main.rs`)

The `Movix` struct maintains:

- Navigation and page state
- Hero content with video frame handles
- Content sections with scroll offsets
- Search results with filter state
- Detail popup data with season/episode selection
- Multiple video player instances (hero, card, detail, movie)
- Image and trailer URL caches

---

## Message Flow

```rust
pub enum Message {
    // Navigation
    NavigateTo(Page),
    SearchQueryChanged(String),

    // Content Loading
    ContentLoaded(Result<Vec<ContentSection>, ApiError>),
    HeroLoaded(Result<MediaItem, ApiError>),
    ImageLoaded(String, Result<Handle, String>),

    // Interaction
    HoverCard(Option<MediaId>),
    PlayContent(MediaId),
    OpenDetailPopup(MediaId),

    // Video Playback
    TrailerStreamUrlLoaded(MediaId, Result<String, String>),
    HeroFrameTick,
    MoviePlayerSeek(f64),

    // Filters
    SetMediaTypeFilter(MediaTypeFilter),
    SetGenreFilter(Option<u64>),
    SetMinRating(f32),
}
```

---

## Dependencies

| Crate           | Purpose                                |
| --------------- | -------------------------------------- |
| `iced`          | GUI framework with async runtime       |
| `gstreamer`     | Video decoding and playback            |
| `gstreamer-app` | Frame extraction via appsink           |
| `reqwest`       | HTTP client for API and image fetching |
| `serde`         | JSON serialization                     |
| `tokio`         | Async runtime                          |
| `image`         | Color palette extraction               |
| `base64`        | Stream URL deobfuscation               |
| `regex`         | HTML parsing for stream resolution     |
| `async-trait`   | Async trait support                    |

---

## Build

### Prerequisites

- Rust 1.70+
- GStreamer development libraries
- yt-dlp (auto-downloaded during build)

### Linux (Debian/Ubuntu)

```bash
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly
```

### Compile

```bash
cargo build --release
```

---

## Usage

```bash
./target/release/movix
```

On first launch, you'll be prompted to enter your TMDB API key and language preference. Get a free API key at [themoviedb.org/settings/api](https://www.themoviedb.org/settings/api).

### Controls

- Scroll horizontally through content sections
- Hover cards to preview trailers
- Click cards or "More Info" for detail popup
- Use search bar with filters for discovery
- Full-screen player: Space (play/pause), Arrow keys (seek), M (mute)

---

## Publish

```bash
git tag -a v1.0.0 -m "Release version 1.0.0"
git push origin v1.0.0
```

---

## License

Movix: A native desktop streaming client.
Copyright (C) 2025 SapphireBluet

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
