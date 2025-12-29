use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use iced::widget::image::Handle;
use iced::Color;
use serde::Deserialize;

fn simple_hash(s: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
}

fn get_cache_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home)
            .join(".cache")
            .join("movix")
            .join("images")
    })
}

pub const BACKGROUND_BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const SURFACE_DARK_GRAY: Color = Color::from_rgb(0.078, 0.078, 0.078);
pub const NETFLIX_RED: Color = Color::from_rgb(0.898, 0.035, 0.078);
pub const TEXT_WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);
pub const TEXT_GRAY: Color = Color::from_rgb(0.702, 0.702, 0.702);

pub const SECTION_IDS: [&str; 10] = [
    "section-0",
    "section-1",
    "section-2",
    "section-3",
    "section-4",
    "section-5",
    "section-6",
    "section-7",
    "section-8",
    "section-9",
];

pub fn section_id(index: usize) -> Option<&'static str> {
    SECTION_IDS.get(index).copied()
}

pub type MediaId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home,
    Series,
    Movies,
    MostRecent,
    MyList,
    Detail(MediaId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NavItem {
    Home,
    Series,
    Movies,
    MostRecent,
    MyList,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    Idle,
    Loading,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum MediaType {
    Movie,
    TvSeries,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Genre {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaTypeFilter {
    #[default]
    All,
    Movies,
    TvSeries,
}

impl std::fmt::Display for MediaTypeFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaTypeFilter::All => write!(f, "All"),
            MediaTypeFilter::Movies => write!(f, "Movies"),
            MediaTypeFilter::TvSeries => write!(f, "Series"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOption {
    #[default]
    Popularity,
    Rating,
    ReleaseDate,
    Alphabetical,
}

impl std::fmt::Display for SortOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOption::Popularity => write!(f, "Popularity"),
            SortOption::Rating => write!(f, "Rating"),
            SortOption::ReleaseDate => write!(f, "Release Date"),
            SortOption::Alphabetical => write!(f, "A-Z"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub media_type: MediaTypeFilter,
    pub genre_id: Option<u64>,
    pub year_from: Option<u32>,
    pub year_to: Option<u32>,
    pub min_rating: f32,
    pub sort_by: SortOption,
}

impl SearchFilters {
    pub fn apply(&self, items: &[MediaItem]) -> Vec<MediaItem> {
        let mut filtered: Vec<MediaItem> = items
            .iter()
            .filter(|item| self.matches(item))
            .cloned()
            .collect();
        self.sort(&mut filtered);
        filtered
    }

    fn matches(&self, item: &MediaItem) -> bool {
        self.matches_media_type(item)
            && self.matches_genre(item)
            && self.matches_year_range(item)
            && self.matches_rating(item)
    }

    fn matches_media_type(&self, item: &MediaItem) -> bool {
        match self.media_type {
            MediaTypeFilter::All => true,
            MediaTypeFilter::Movies => matches!(item.media_type, MediaType::Movie),
            MediaTypeFilter::TvSeries => matches!(item.media_type, MediaType::TvSeries),
        }
    }

    fn matches_genre(&self, item: &MediaItem) -> bool {
        match self.genre_id {
            None => true,
            Some(id) => item.genres.iter().any(|g| g.id == id),
        }
    }

    fn matches_year_range(&self, item: &MediaItem) -> bool {
        let year = item
            .release_date
            .as_ref()
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse::<u32>().ok());

        let (from, to) = self.normalized_year_range();
        match (year, from, to) {
            (None, _, _) => true,
            (Some(y), Some(f), Some(t)) => y >= f && y <= t,
            (Some(y), Some(f), None) => y >= f,
            (Some(y), None, Some(t)) => y <= t,
            (Some(_), None, None) => true,
        }
    }

    fn normalized_year_range(&self) -> (Option<u32>, Option<u32>) {
        match (self.year_from, self.year_to) {
            (Some(from), Some(to)) if from > to => (Some(to), Some(from)),
            (from, to) => (from, to),
        }
    }

    fn matches_rating(&self, item: &MediaItem) -> bool {
        item.vote_average >= self.min_rating
    }

    fn sort(&self, items: &mut [MediaItem]) {
        match self.sort_by {
            SortOption::Popularity | SortOption::Rating => {
                items.sort_by(|a, b| {
                    b.vote_average
                        .partial_cmp(&a.vote_average)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortOption::ReleaseDate => {
                items.sort_by(|a, b| b.release_date.cmp(&a.release_date));
            }
            SortOption::Alphabetical => {
                items.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CastMember {
    pub id: u64,
    pub name: String,
    pub character: String,
    pub profile_path: Option<String>,
    pub order: u32,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: u64,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub parts: Vec<MediaItem>,
}

#[derive(Debug, Clone, Default)]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
    pub facebook_id: Option<String>,
    pub twitter_id: Option<String>,
    pub instagram_id: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Keyword {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ProductionCompany {
    pub id: u64,
    pub name: String,
    pub logo_path: Option<String>,
    pub origin_country: String,
}

#[derive(Debug, Clone)]
pub struct Season {
    pub id: u64,
    pub season_number: u32,
    pub name: String,
    pub episode_count: u32,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: u64,
    pub episode_number: u32,
    pub season_number: u32,
    pub name: String,
    pub overview: String,
    pub air_date: Option<String>,
    pub still_path: Option<String>,
    pub runtime: Option<u32>,
    pub vote_average: f32,
}

#[derive(Debug, Clone)]
pub struct DetailPopupData {
    pub media_item: MediaItem,
    pub cast: Vec<CastMember>,
    pub collection: Option<Collection>,
    pub similar: Vec<MediaItem>,
    pub external_ids: ExternalIds,
    pub keywords: Vec<Keyword>,
    pub production_companies: Vec<ProductionCompany>,
    pub seasons: Vec<Season>,
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub id: MediaId,
    pub title: String,
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub logo_path: Option<String>,
    pub media_type: MediaType,
    pub vote_average: f32,
    pub release_date: Option<String>,
    pub runtime: Option<u32>,
    pub certification: Option<String>,
    pub tagline: Option<String>,
    pub genres: Vec<Genre>,
    pub budget: Option<u64>,
    pub revenue: Option<u64>,
    pub status: Option<String>,
    pub original_language: Option<String>,
    pub collection_id: Option<u64>,
    pub number_of_episodes: Option<u32>,
    pub number_of_seasons: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    Trending,
    TopRated,
    MostRecent,
    Action,
    Comedy,
    Drama,
    Series,
    Recommended,
}

#[derive(Debug, Clone)]
pub struct ContentSection {
    pub title: String,
    pub category: Category,
    pub items: Vec<MediaItem>,
}

#[derive(Debug, Clone)]
pub struct HeaderState {
    pub active_nav: NavItem,
    pub search_focused: bool,
}

impl Default for HeaderState {
    fn default() -> Self {
        Self {
            active_nav: NavItem::Home,
            search_focused: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImageCache {
    cache: HashMap<String, Handle>,
    pending: HashSet<String>,
    cache_directory: Option<PathBuf>,
}

impl ImageCache {
    pub fn new() -> Self {
        let cache_directory = get_cache_dir();
        if let Some(ref dir) = cache_directory {
            let _ = std::fs::create_dir_all(dir);
        }
        Self {
            cache: HashMap::new(),
            pending: HashSet::new(),
            cache_directory,
        }
    }

    pub fn get(&self, url: &str) -> Option<&Handle> {
        self.cache.get(url)
    }

    pub fn insert(&mut self, url: String, handle: Handle) {
        self.pending.remove(&url);
        self.cache.insert(url, handle);
    }

    pub fn is_pending(&self, url: &str) -> bool {
        self.pending.contains(url)
    }

    pub fn mark_pending(&mut self, url: String) {
        self.pending.insert(url);
    }

    pub fn get_cache_path(&self, url: &str) -> Option<PathBuf> {
        self.cache_directory
            .as_ref()
            .map(|dir| dir.join(simple_hash(url)))
    }
}

#[derive(Debug, Clone)]
pub enum ApiError {
    Network(String),
    Parse(String),
    RateLimit,
    Unauthorized,
}

#[derive(Debug, Clone)]
pub enum ProfileAction {
    OpenSettings,
    OpenProfileSettings,
    SwitchProfile(u64),
}

#[derive(Debug, Clone)]
pub enum ScrollDirection {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum Message {
    Setup(crate::settings::SetupMessage),
    NavigateTo(Page),
    SearchQueryChanged(String),
    SearchSubmit,
    SearchResultsLoaded(Result<Vec<MediaItem>, ApiError>),
    ToggleProfileMenu,
    CloseProfileMenu,
    ProfileAction(ProfileAction),
    PlayContent(MediaId),
    ShowMoreInfo(MediaId),
    HoverCard(Option<MediaId>),
    HoverCardDelayed(MediaId),
    HoverSection(Option<usize>),
    ContentLoaded(Result<Vec<ContentSection>, ApiError>),
    HeroLoaded(Result<MediaItem, ApiError>),
    ImageLoaded(String, Result<Handle, String>),
    LoadImage(String),
    LogoLoaded(MediaId, Result<Option<String>, ApiError>),
    RetryLoad,
    ScrollSection(usize, ScrollDirection),
    SectionScrolled(usize, f32),
    AnimateScroll(usize),
    TrailerVideosLoaded(MediaId, Result<Vec<crate::video::TrailerVideo>, ApiError>),
    TrailerStreamUrlLoaded(MediaId, Result<String, String>),
    TrailerStreamUrlPreloaded(MediaId, Result<String, String>),
    HeroFrameTick,
    CardFrameTick,
    StopCardTrailer,
    PauseHeroTrailer,
    ResumeHeroTrailer,
    HeroVisibilityChanged(bool),
    MainScrolled(f32),
    ToggleHeroMute,
    ReplayHeroTrailer,
    HeroVideoEnded,
    MoviePlayerOpen(MediaId, String),
    MoviePlayerClose,
    MoviePlayerTogglePlay,
    MoviePlayerSeek(f64),
    MoviePlayerSeekRelative(f64),
    MoviePlayerSetVolume(f64),
    MoviePlayerToggleMute,
    MoviePlayerToggleFullscreen,
    MoviePlayerFrameTick,
    MoviePlayerStreamResolved(MediaId, Result<String, String>),
    MoviePlayerShowControls,
    MoviePlayerHideControls,
    OpenDetailPopup(MediaId),
    CloseDetailPopup,
    DetailDataLoaded(Result<DetailPopupData, ApiError>),
    DetailSelectSeason(Option<u32>),
    DetailEpisodesLoaded(Result<Vec<Episode>, ApiError>),
    DetailHoverCard(Option<MediaId>),
    DetailHoverCardDelayed(MediaId),
    DetailFrameTick,
    DetailTrailerLoaded(MediaId, Result<String, String>),
    SearchDebounceTriggered,
    ClearSearch,
    SetMediaTypeFilter(MediaTypeFilter),
    SetGenreFilter(Option<u64>),
    SetYearFrom(Option<u32>),
    SetYearTo(Option<u32>),
    SetMinRating(f32),
    SetSortOption(SortOption),
    ResetFilters,
    GenresLoaded(Result<Vec<Genre>, ApiError>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbMediaResult {
    pub id: u64,
    pub title: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub media_type: Option<String>,
    #[serde(default)]
    pub vote_average: f32,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
}

impl From<TmdbMediaResult> for MediaItem {
    fn from(result: TmdbMediaResult) -> Self {
        let media_type = match result.media_type.as_deref() {
            Some("tv") => MediaType::TvSeries,
            _ => MediaType::Movie,
        };
        Self {
            id: result.id,
            title: result.title.or(result.name).unwrap_or_default(),
            overview: result.overview,
            poster_path: result.poster_path,
            backdrop_path: result.backdrop_path,
            logo_path: None,
            media_type,
            vote_average: result.vote_average,
            release_date: result.release_date.or(result.first_air_date),
            runtime: None,
            certification: None,
            tagline: None,
            genres: Vec::new(),
            budget: None,
            revenue: None,
            status: None,
            original_language: None,
            collection_id: None,
            number_of_episodes: None,
            number_of_seasons: None,
        }
    }
}

#[derive(Deserialize)]
pub struct TmdbSearchResponse {
    pub results: Vec<TmdbMediaResult>,
}

pub fn truncate_description(description: &str, max_length: usize) -> String {
    if description.len() <= max_length {
        return description.to_string();
    }
    let truncated = &description[..max_length];
    format!(
        "{}...",
        truncated.rfind(' ').map_or(truncated, |i| &truncated[..i])
    )
}
