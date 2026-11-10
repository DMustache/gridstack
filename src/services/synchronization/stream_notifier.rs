use tokio::sync::watch;

/// Publishes the latest committed position of the shared sync stream so that
/// waiting `/sync` long-polls wake up as soon as new data is available.
///
/// In-process only: every writer that advances the shared stream publishes
/// here after its transaction commits.
pub struct StreamNotifier {
    latest_stream_position_sender: watch::Sender<i64>,
}

impl StreamNotifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            latest_stream_position_sender: watch::Sender::new(0),
        }
    }

    /// Records a newly committed stream position. Positions never move
    /// backwards, so a late publisher with an older position wakes nobody.
    pub fn publish_stream_position(&self, committed_stream_position: i64) {
        self.latest_stream_position_sender
            .send_if_modified(|latest_stream_position| {
                if committed_stream_position > *latest_stream_position {
                    *latest_stream_position = committed_stream_position;
                    true
                } else {
                    false
                }
            });
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<i64> {
        self.latest_stream_position_sender.subscribe()
    }
}

impl Default for StreamNotifier {
    fn default() -> Self {
        Self::new()
    }
}
