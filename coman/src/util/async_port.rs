struct AsyncPort {
    tempfile: Arc<NamedTempFile>,
}

impl AsyncPort {
    pub fn new() -> Self {
        let tempfile = Arc::new(NamedTempFile::new().unwrap());
        Self { tempfile }
    }

    pub async fn write_file(&self) -> Duration {
        let t_start = std::time::Instant::now();
        // Write to file
        let mut file = tokio::fs::File::create(self.tempfile.path()).await.unwrap();
        file.write_all(b"Hello, world!").await.unwrap();

        t_start.elapsed()
    }
}

#[tuirealm::async_trait]
impl PollAsync<UserEvent> for AsyncPort {
    async fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        let result = self.write_file().await;

        Ok(Some(Event::User(UserEvent::WroteFile(result))))
    }
}
