pub mod providers;
pub mod resolvers;

use async_trait::async_trait;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum StreamError {
    Network(String),
    Parse(String),
    NotFound(String),
    Config(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Network(msg) => write!(f, "Network error: {}", msg),
            StreamError::Parse(msg) => write!(f, "Parse error: {}", msg),
            StreamError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StreamError::Config(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for StreamError {}

#[async_trait]
pub trait StreamProvider: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Get the stream page URL for a given title
    async fn get_stream_page_url(&self, title: &str) -> Result<String, StreamError>;
}

#[async_trait]
pub trait StreamResolver: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Check if this resolver can handle the given URL
    fn can_handle(&self, url: &str) -> bool;

    /// Resolve a stream page URL to a direct playable URL
    async fn resolve(&self, url: &str) -> Result<String, StreamError>;
}

/// Combined service that uses providers and resolvers together
pub struct StreamingService {
    providers: Vec<Box<dyn StreamProvider>>,
    resolvers: Vec<Box<dyn StreamResolver>>,
}

impl StreamingService {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            resolvers: Vec::new(),
        }
    }

    /// Add a stream provider
    pub fn add_provider<P: StreamProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }

    /// Add a stream resolver
    pub fn add_resolver<R: StreamResolver + 'static>(&mut self, resolver: R) {
        self.resolvers.push(Box::new(resolver));
    }

    /// Get a direct stream URL for a title using the first available provider
    pub async fn get_stream_url(&self, title: &str) -> Result<String, StreamError> {
        // Try each provider until one succeeds
        let mut last_error = StreamError::NotFound("No providers available".to_string());

        for provider in &self.providers {
            match provider.get_stream_page_url(title).await {
                Ok(page_url) => {
                    // Find a resolver that can handle this URL
                    for resolver in &self.resolvers {
                        if resolver.can_handle(&page_url) {
                            match resolver.resolve(&page_url).await {
                                Ok(stream_url) => return Ok(stream_url),
                                Err(e) => last_error = e,
                            }
                        }
                    }
                }
                Err(e) => last_error = e,
            }
        }

        Err(last_error)
    }

    #[allow(dead_code)]
    pub async fn get_stream_url_with_provider(
        &self,
        title: &str,
        provider_name: &str,
    ) -> Result<String, StreamError> {
        let provider = self
            .providers
            .iter()
            .find(|p| p.name() == provider_name)
            .ok_or_else(|| {
                StreamError::NotFound(format!("Provider '{}' not found", provider_name))
            })?;

        let page_url = provider.get_stream_page_url(title).await?;

        for resolver in &self.resolvers {
            if resolver.can_handle(&page_url) {
                return resolver.resolve(&page_url).await;
            }
        }

        Err(StreamError::NotFound(
            "No resolver found for URL".to_string(),
        ))
    }

    #[allow(dead_code)]
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.name()).collect()
    }

    #[allow(dead_code)]
    pub fn resolver_names(&self) -> Vec<&str> {
        self.resolvers.iter().map(|r| r.name()).collect()
    }
}

impl Default for StreamingService {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_default_service() -> StreamingService {
    let mut service = StreamingService::new();
    service.add_provider(providers::FilmpalastToProvider::new());
    service.add_resolver(resolvers::voe::VoeResolver::new());
    service
}
