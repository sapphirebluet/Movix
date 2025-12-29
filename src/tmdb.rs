use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::settings::AppSettings;

use crate::media::{
    ApiError, CastMember, Category, Collection, ContentSection, DetailPopupData, Episode,
    ExternalIds, Genre, Keyword, MediaId, MediaItem, MediaType, ProductionCompany, Season,
    TmdbMediaResult, TmdbSearchResponse,
};
use crate::video::{TrailerVideo, VideosResponse};

use serde::Deserialize;

const CACHE_TTL_SECONDS: u64 = 300;

fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenreListResponse {
    pub genres: Vec<Genre>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbCreditsResponse {
    pub cast: Vec<TmdbCastMember>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbCastMember {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub character: String,
    pub profile_path: Option<String>,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbExternalIdsResponse {
    pub imdb_id: Option<String>,
    pub facebook_id: Option<String>,
    pub twitter_id: Option<String>,
    pub instagram_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbKeywordsResponse {
    pub keywords: Option<Vec<TmdbKeyword>>,
    pub results: Option<Vec<TmdbKeyword>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbKeyword {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbCollectionResponse {
    pub id: u64,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub parts: Vec<TmdbMediaResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbSeasonResponse {
    #[serde(default)]
    pub episodes: Vec<TmdbEpisode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmdbEpisode {
    pub id: u64,
    pub episode_number: u32,
    pub season_number: u32,
    pub name: String,
    #[serde(default)]
    pub overview: String,
    pub air_date: Option<String>,
    pub still_path: Option<String>,
    pub runtime: Option<u32>,
    #[serde(default)]
    pub vote_average: f32,
}

pub async fn fetch_image_bytes(url: String) -> Result<Vec<u8>, String> {
    reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

#[derive(Clone)]
pub enum ImageSize {
    Poster,
    Backdrop,
    Original,
}

struct CacheEntry<T> {
    data: T,
    created_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            created_at: Instant::now(),
        }
    }

    fn is_valid(&self) -> bool {
        self.created_at.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS)
    }
}

#[derive(Clone)]
pub struct TmdbClient {
    api_key: String,
    base_url: String,
    image_base_url: String,
    language: String,
    http_client: Arc<reqwest::Client>,
    list_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<MediaItem>>>>>,
    details_cache: Arc<RwLock<HashMap<String, CacheEntry<MediaItem>>>>,
    detail_popup_cache: Arc<RwLock<HashMap<String, CacheEntry<DetailPopupData>>>>,
}

impl TmdbClient {
    pub fn new(api_key: String, language: String) -> Self {
        Self {
            api_key,
            base_url: String::from("https://api.themoviedb.org/3"),
            image_base_url: String::from("https://image.tmdb.org/t/p"),
            language,
            http_client: Arc::new(reqwest::Client::new()),
            list_cache: Arc::new(RwLock::new(HashMap::new())),
            details_cache: Arc::new(RwLock::new(HashMap::new())),
            detail_popup_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn from_settings(settings: &AppSettings) -> Self {
        let language = if settings.language.is_empty() {
            String::from("en-US")
        } else {
            settings.language.clone()
        };
        Self::new(settings.api_key.clone(), language)
    }

    pub fn image_url(&self, path: &str, size: ImageSize) -> String {
        let size_path = match size {
            ImageSize::Poster => "w500",
            ImageSize::Backdrop | ImageSize::Original => "original",
        };
        format!("{}/{}{}", self.image_base_url, size_path, path)
    }

    fn build_url(&self, endpoint: &str) -> String {
        format!(
            "{}{}?api_key={}&language={}",
            self.base_url, endpoint, self.api_key, self.language
        )
    }

    fn build_url_with_params(&self, endpoint: &str, params: &str) -> String {
        format!("{}&{}", self.build_url(endpoint), params)
    }

    fn get_cached_list(&self, key: &str) -> Option<Vec<MediaItem>> {
        self.list_cache
            .read()
            .ok()?
            .get(key)
            .filter(|e| e.is_valid())
            .map(|e| e.data.clone())
    }

    fn set_cached_list(&self, key: String, data: Vec<MediaItem>) {
        if let Ok(mut cache) = self.list_cache.write() {
            cache.insert(key, CacheEntry::new(data));
        }
    }

    fn get_cached_details(&self, key: &str) -> Option<MediaItem> {
        self.details_cache
            .read()
            .ok()?
            .get(key)
            .filter(|e| e.is_valid())
            .map(|e| e.data.clone())
    }

    fn set_cached_details(&self, key: String, data: MediaItem) {
        if let Ok(mut cache) = self.details_cache.write() {
            cache.insert(key, CacheEntry::new(data));
        }
    }

    fn get_cached_popup(&self, key: &str) -> Option<DetailPopupData> {
        self.detail_popup_cache
            .read()
            .ok()?
            .get(key)
            .filter(|e| e.is_valid())
            .map(|e| e.data.clone())
    }

    fn set_cached_popup(&self, key: String, data: DetailPopupData) {
        if let Ok(mut cache) = self.detail_popup_cache.write() {
            cache.insert(key, CacheEntry::new(data));
        }
    }

    async fn fetch_response(&self, url: &str) -> Result<reqwest::Response, ApiError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        match response.status().as_u16() {
            401 => Err(ApiError::Unauthorized),
            429 => Err(ApiError::RateLimit),
            s if s >= 400 => Err(ApiError::Network(format!("HTTP error: {}", s))),
            _ => Ok(response),
        }
    }

    async fn fetch_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, ApiError> {
        self.fetch_response(url)
            .await?
            .json()
            .await
            .map_err(|e| ApiError::Parse(e.to_string()))
    }

    async fn fetch_and_parse(
        &self,
        url: &str,
        cache_key: &str,
    ) -> Result<Vec<MediaItem>, ApiError> {
        if let Some(cached) = self.get_cached_list(cache_key) {
            return Ok(cached);
        }

        let response: TmdbSearchResponse = self.fetch_json(url).await?;
        let items: Vec<MediaItem> = response.results.into_iter().map(MediaItem::from).collect();
        self.set_cached_list(cache_key.to_string(), items.clone());
        Ok(items)
    }

    pub async fn fetch_trending(&self) -> Result<Vec<MediaItem>, ApiError> {
        self.fetch_and_parse(&self.build_url("/trending/all/week"), "trending")
            .await
    }

    pub async fn fetch_top_rated_movies(&self) -> Result<Vec<MediaItem>, ApiError> {
        self.fetch_and_parse(&self.build_url("/movie/top_rated"), "top_rated_movies")
            .await
    }

    pub async fn fetch_top_rated_series(&self) -> Result<Vec<MediaItem>, ApiError> {
        self.fetch_and_parse(&self.build_url("/tv/top_rated"), "top_rated_series")
            .await
    }

    pub async fn fetch_by_genre(
        &self,
        genre_id: u32,
        media_type: &str,
    ) -> Result<Vec<MediaItem>, ApiError> {
        let cache_key = format!("genre_{}_{}", genre_id, media_type);
        let url = self.build_url_with_params(
            &format!("/discover/{}", media_type),
            &format!("with_genres={}&sort_by=popularity.desc", genre_id),
        );
        self.fetch_and_parse(&url, &cache_key).await
    }

    pub async fn search(&self, query: &str) -> Result<Vec<MediaItem>, ApiError> {
        let cache_key = format!("search_{}", query);
        let url =
            self.build_url_with_params("/search/multi", &format!("query={}", url_encode(query)));
        self.fetch_and_parse(&url, &cache_key).await
    }

    pub async fn fetch_genres(&self) -> Result<Vec<Genre>, ApiError> {
        let movie_url = self.build_url("/genre/movie/list");
        let tv_url = self.build_url("/genre/tv/list");

        let movie_response: GenreListResponse = self.fetch_json(&movie_url).await?;
        let tv_response: GenreListResponse = self.fetch_json(&tv_url).await?;

        let mut genres = movie_response.genres;
        for tv_genre in tv_response.genres {
            if !genres.iter().any(|g| g.id == tv_genre.id) {
                genres.push(tv_genre);
            }
        }
        genres.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(genres)
    }

    pub async fn fetch_full_media_details(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<MediaItem, ApiError> {
        let cache_key = format!("details_{:?}_{}", media_type, id);
        if let Some(cached) = self.get_cached_details(&cache_key) {
            return Ok(cached);
        }

        let type_path = media_type_path(media_type);
        let append = match media_type {
            MediaType::Movie => "videos,images,release_dates",
            MediaType::TvSeries => "videos,images,content_ratings",
        };

        let url = self.build_url_with_params(
            &format!("/{}/{}", type_path, id),
            &format!(
                "append_to_response={}&include_image_language=en,null",
                append
            ),
        );

        let json: serde_json::Value = self.fetch_json(&url).await?;
        let result: TmdbMediaResult =
            serde_json::from_value(json.clone()).map_err(|e| ApiError::Parse(e.to_string()))?;

        let mut item = MediaItem::from(result);
        item.runtime = extract_runtime(&json, media_type);
        item.certification = extract_certification(&json, media_type);
        item.logo_path = extract_logo_path(&json);

        self.set_cached_details(cache_key, item.clone());
        Ok(item)
    }

    pub async fn fetch_videos(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<Vec<TrailerVideo>, ApiError> {
        let url = self.build_url(&format!("/{}/{}/videos", media_type_path(media_type), id));
        let response: VideosResponse = self.fetch_json(&url).await?;
        Ok(response.results)
    }

    pub async fn fetch_movie_details(&self, id: MediaId) -> Result<MediaItem, ApiError> {
        self.fetch_full_media_details(id, &MediaType::Movie).await
    }

    pub async fn fetch_media_details(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<(Option<u32>, Option<String>), ApiError> {
        let item = self.fetch_full_media_details(id, media_type).await?;
        Ok((item.runtime, item.certification))
    }

    pub async fn fetch_media_images(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<Option<String>, ApiError> {
        let item = self.fetch_full_media_details(id, media_type).await?;
        Ok(item.logo_path)
    }

    pub async fn fetch_credits(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<Vec<CastMember>, ApiError> {
        let url = self.build_url(&format!("/{}/{}/credits", media_type_path(media_type), id));
        let credits: TmdbCreditsResponse = self.fetch_json(&url).await?;
        Ok(credits
            .cast
            .into_iter()
            .map(|c| CastMember {
                id: c.id,
                name: c.name,
                character: c.character,
                profile_path: c.profile_path,
                order: c.order,
            })
            .collect())
    }

    pub async fn fetch_external_ids(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<ExternalIds, ApiError> {
        let url = self.build_url(&format!(
            "/{}/{}/external_ids",
            media_type_path(media_type),
            id
        ));
        let ids: TmdbExternalIdsResponse = self.fetch_json(&url).await?;
        Ok(ExternalIds {
            imdb_id: ids.imdb_id,
            facebook_id: ids.facebook_id,
            twitter_id: ids.twitter_id,
            instagram_id: ids.instagram_id,
            homepage: None,
        })
    }

    pub async fn fetch_keywords(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<Vec<Keyword>, ApiError> {
        let url = self.build_url(&format!("/{}/{}/keywords", media_type_path(media_type), id));
        let response: TmdbKeywordsResponse = self.fetch_json(&url).await?;
        let keywords = response.keywords.or(response.results).unwrap_or_default();
        Ok(keywords
            .into_iter()
            .map(|k| Keyword {
                id: k.id,
                name: k.name,
            })
            .collect())
    }

    pub async fn fetch_collection(&self, id: u64) -> Result<Collection, ApiError> {
        let url = self.build_url_with_params(
            &format!("/collection/{}", id),
            "append_to_response=images&include_image_language=en,null",
        );
        let json: serde_json::Value = self.fetch_json(&url).await?;
        let collection: TmdbCollectionResponse =
            serde_json::from_value(json.clone()).map_err(|e| ApiError::Parse(e.to_string()))?;

        let parts_with_logos = self
            .fetch_collection_parts_with_logos(&collection.parts)
            .await;

        Ok(Collection {
            id: collection.id,
            name: collection.name,
            poster_path: collection.poster_path,
            backdrop_path: collection.backdrop_path,
            parts: parts_with_logos,
        })
    }

    async fn fetch_collection_parts_with_logos(&self, parts: &[TmdbMediaResult]) -> Vec<MediaItem> {
        let mut results = Vec::with_capacity(parts.len());
        for part in parts {
            let mut item = MediaItem::from(part.clone());
            if let Ok(details) = self
                .fetch_full_media_details(item.id, &item.media_type)
                .await
            {
                item.logo_path = details.logo_path;
            }
            results.push(item);
        }
        results
    }

    async fn fetch_similar_with_logos(&self, items: &[MediaItem]) -> Vec<MediaItem> {
        let mut results = Vec::with_capacity(items.len().min(3));
        for item in items.iter().take(3) {
            let mut result = item.clone();
            if result.logo_path.is_none() {
                if let Ok(details) = self
                    .fetch_full_media_details(item.id, &item.media_type)
                    .await
                {
                    result.logo_path = details.logo_path;
                }
            }
            results.push(result);
        }
        results
    }

    pub async fn fetch_recommendations(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<Vec<MediaItem>, ApiError> {
        let type_path = media_type_path(media_type);
        let cache_key = format!("recommendations_{}_{}", type_path, id);
        let url = self.build_url(&format!("/{}/{}/recommendations", type_path, id));
        self.fetch_and_parse(&url, &cache_key).await
    }

    pub async fn fetch_season_episodes(
        &self,
        tv_id: MediaId,
        season_number: u32,
    ) -> Result<Vec<Episode>, ApiError> {
        let url = self.build_url(&format!("/tv/{}/season/{}", tv_id, season_number));
        let season: TmdbSeasonResponse = self.fetch_json(&url).await?;
        Ok(season
            .episodes
            .into_iter()
            .map(|e| Episode {
                id: e.id,
                episode_number: e.episode_number,
                season_number: e.season_number,
                name: e.name,
                overview: e.overview,
                air_date: e.air_date,
                still_path: e.still_path,
                runtime: e.runtime,
                vote_average: e.vote_average,
            })
            .collect())
    }

    pub async fn fetch_detail_popup_data(
        &self,
        id: MediaId,
        media_type: &MediaType,
    ) -> Result<DetailPopupData, ApiError> {
        let cache_key = format!("popup_{:?}_{}", media_type, id);
        if let Some(cached) = self.get_cached_popup(&cache_key) {
            return Ok(cached);
        }

        let type_path = media_type_path(media_type);
        let append = match media_type {
            MediaType::Movie => {
                "videos,images,release_dates,credits,external_ids,keywords,recommendations"
            }
            MediaType::TvSeries => {
                "videos,images,content_ratings,credits,external_ids,keywords,recommendations"
            }
        };

        let url = self.build_url_with_params(
            &format!("/{}/{}", type_path, id),
            &format!(
                "append_to_response={}&include_image_language=en,null",
                append
            ),
        );

        let json: serde_json::Value = self.fetch_json(&url).await?;
        let result: TmdbMediaResult =
            serde_json::from_value(json.clone()).map_err(|e| ApiError::Parse(e.to_string()))?;

        let mut item = MediaItem::from(result);
        populate_media_item(&mut item, &json, media_type);

        let cast = parse_credits(&json);
        let mut external_ids = parse_external_ids(&json);
        external_ids.homepage = json
            .get("homepage")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let keywords = parse_keywords(&json);
        let similar_raw = parse_recommendations(&json);
        let production_companies = parse_production_companies(&json);
        let seasons = parse_seasons(&json);

        let similar = self.fetch_similar_with_logos(&similar_raw).await;

        let collection = if let Some(collection_id) = item.collection_id {
            self.fetch_collection(collection_id).await.ok()
        } else {
            None
        };

        let data = DetailPopupData {
            media_item: item,
            cast,
            collection,
            similar,
            external_ids,
            keywords,
            production_companies,
            seasons,
        };

        self.set_cached_popup(cache_key, data.clone());
        Ok(data)
    }
}

fn media_type_path(media_type: &MediaType) -> &'static str {
    match media_type {
        MediaType::Movie => "movie",
        MediaType::TvSeries => "tv",
    }
}

fn extract_runtime(json: &serde_json::Value, media_type: &MediaType) -> Option<u32> {
    match media_type {
        MediaType::Movie => json.get("runtime")?.as_u64().map(|v| v as u32),
        MediaType::TvSeries => json
            .get("episode_run_time")?
            .as_array()?
            .first()?
            .as_u64()
            .map(|v| v as u32),
    }
}

fn extract_certification(json: &serde_json::Value, media_type: &MediaType) -> Option<String> {
    let (key, field) = match media_type {
        MediaType::Movie => ("release_dates", "certification"),
        MediaType::TvSeries => ("content_ratings", "rating"),
    };

    let results = json.get(key)?.get("results")?.as_array()?;
    let us_entry = results
        .iter()
        .find(|r| r.get("iso_3166_1").and_then(|v| v.as_str()) == Some("US"))?;

    let cert = match media_type {
        MediaType::Movie => us_entry
            .get("release_dates")?
            .as_array()?
            .first()?
            .get(field)?,
        MediaType::TvSeries => us_entry.get(field)?,
    };

    cert.as_str().filter(|s| !s.is_empty()).map(String::from)
}

fn extract_logo_path(json: &serde_json::Value) -> Option<String> {
    let logos = json.get("images")?.get("logos")?.as_array()?;
    logos
        .iter()
        .find(|l| l.get("iso_639_1").and_then(|v| v.as_str()) == Some("en"))
        .or_else(|| logos.first())
        .and_then(|l| l.get("file_path"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn populate_media_item(item: &mut MediaItem, json: &serde_json::Value, media_type: &MediaType) {
    item.runtime = extract_runtime(json, media_type);
    item.certification = extract_certification(json, media_type);
    item.logo_path = extract_logo_path(json);
    item.tagline = json
        .get("tagline")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    item.status = json
        .get("status")
        .and_then(|v| v.as_str())
        .map(String::from);
    item.original_language = json
        .get("original_language")
        .and_then(|v| v.as_str())
        .map(String::from);
    item.budget = json.get("budget").and_then(|v| v.as_u64());
    item.revenue = json.get("revenue").and_then(|v| v.as_u64());
    item.number_of_episodes = json
        .get("number_of_episodes")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    item.number_of_seasons = json
        .get("number_of_seasons")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    if let Some(genres) = json.get("genres").and_then(|v| v.as_array()) {
        item.genres = genres
            .iter()
            .filter_map(|g| {
                Some(Genre {
                    id: g.get("id")?.as_u64()?,
                    name: g.get("name")?.as_str()?.to_string(),
                })
            })
            .collect();
    }

    if let Some(belongs_to) = json.get("belongs_to_collection") {
        item.collection_id = belongs_to.get("id").and_then(|v| v.as_u64());
    }
}

fn parse_credits(json: &serde_json::Value) -> Vec<CastMember> {
    json.get("credits")
        .and_then(|c| c.get("cast"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    Some(CastMember {
                        id: c.get("id")?.as_u64()?,
                        name: c.get("name")?.as_str()?.to_string(),
                        character: c
                            .get("character")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        profile_path: c
                            .get("profile_path")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        order: c.get("order").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_external_ids(json: &serde_json::Value) -> ExternalIds {
    json.get("external_ids")
        .map(|ids| ExternalIds {
            imdb_id: ids
                .get("imdb_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            facebook_id: ids
                .get("facebook_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            twitter_id: ids
                .get("twitter_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            instagram_id: ids
                .get("instagram_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            homepage: None,
        })
        .unwrap_or_default()
}

fn parse_keywords(json: &serde_json::Value) -> Vec<Keyword> {
    let keywords_obj = json.get("keywords");
    let arr = keywords_obj
        .and_then(|k| k.get("keywords").or_else(|| k.get("results")))
        .and_then(|v| v.as_array());

    arr.map(|a| {
        a.iter()
            .filter_map(|k| {
                Some(Keyword {
                    id: k.get("id")?.as_u64()?,
                    name: k.get("name")?.as_str()?.to_string(),
                })
            })
            .collect()
    })
    .unwrap_or_default()
}

fn parse_recommendations(json: &serde_json::Value) -> Vec<MediaItem> {
    json.get("recommendations")
        .and_then(|s| s.get("results"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let mut media = serde_json::from_value::<TmdbMediaResult>(item.clone())
                        .ok()
                        .map(MediaItem::from)?;
                    media.logo_path = extract_logo_path(item);
                    Some(media)
                })
                .filter(|item| item.backdrop_path.is_some())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_production_companies(json: &serde_json::Value) -> Vec<ProductionCompany> {
    json.get("production_companies")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    Some(ProductionCompany {
                        id: c.get("id")?.as_u64()?,
                        name: c.get("name")?.as_str()?.to_string(),
                        logo_path: c
                            .get("logo_path")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        origin_country: c
                            .get("origin_country")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_seasons(json: &serde_json::Value) -> Vec<Season> {
    json.get("seasons")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(Season {
                        id: s.get("id")?.as_u64()?,
                        season_number: s.get("season_number")?.as_u64()? as u32,
                        name: s.get("name")?.as_str()?.to_string(),
                        episode_count: s.get("episode_count").and_then(|v| v.as_u64()).unwrap_or(0)
                            as u32,
                        air_date: s.get("air_date").and_then(|v| v.as_str()).map(String::from),
                        poster_path: s
                            .get("poster_path")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub async fn load_initial_content(client: TmdbClient) -> Result<Vec<ContentSection>, ApiError> {
    let trending = client.fetch_trending().await?;
    let top_movies = client.fetch_top_rated_movies().await?;
    let top_series = client.fetch_top_rated_series().await?;
    let action = client.fetch_by_genre(28, "movie").await?;
    let comedy = client.fetch_by_genre(35, "movie").await?;

    Ok(vec![
        ContentSection {
            title: String::from("Top Picks"),
            category: Category::Trending,
            items: trending,
        },
        ContentSection {
            title: String::from("Most Recent"),
            category: Category::TopRated,
            items: top_movies,
        },
        ContentSection {
            title: String::from("Action Movies"),
            category: Category::Action,
            items: action,
        },
        ContentSection {
            title: String::from("Series"),
            category: Category::Series,
            items: top_series,
        },
        ContentSection {
            title: String::from("Recommended"),
            category: Category::Recommended,
            items: comedy,
        },
    ])
}

pub async fn load_hero_content(client: TmdbClient) -> Result<MediaItem, ApiError> {
    let trending = client.fetch_trending().await?;
    let hero = trending
        .into_iter()
        .find(|item| item.backdrop_path.is_some())
        .ok_or_else(|| ApiError::Parse(String::from("No featured content available")))?;

    client
        .fetch_full_media_details(hero.id, &hero.media_type)
        .await
}

pub async fn load_genres(client: TmdbClient) -> Result<Vec<Genre>, ApiError> {
    client.fetch_genres().await
}
