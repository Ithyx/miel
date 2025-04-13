pub(crate) struct ScopeTimer {
    name: String,
    log_level: log::Level,
    timer: std::time::Instant,
}

impl ScopeTimer {
    pub fn new(log_level: log::Level, name: String) -> Self {
        Self {
            name,
            log_level,
            timer: std::time::Instant::now(),
        }
    }
}

impl Drop for ScopeTimer {
    fn drop(&mut self) {
        log::log!(
            self.log_level,
            "{} took {}ms",
            self.name,
            self.timer.elapsed().as_millis()
        );
    }
}

