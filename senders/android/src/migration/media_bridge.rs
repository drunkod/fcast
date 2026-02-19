use gst_app::{AppSink, AppSinkCallbacks, AppSrc};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Forwards samples from one producer `appsink` to many consumer `appsrc`s.
#[derive(Debug, Default)]
pub struct StreamBridge {
    consumers: Arc<Mutex<HashMap<String, AppSrc>>>,
    attached_sink: Option<AppSink>,
}

impl StreamBridge {
    pub fn add_consumer(&mut self, consumer_id: &str, consumer: &AppSrc) {
        self.consumers
            .lock()
            .insert(consumer_id.to_string(), consumer.clone());
    }

    pub fn remove_consumer(&mut self, consumer_id: &str) {
        self.consumers.lock().remove(consumer_id);
    }

    pub fn clear(&mut self) {
        self.consumers.lock().clear();
        if let Some(sink) = self.attached_sink.take() {
            sink.set_callbacks(AppSinkCallbacks::builder().build());
        }
    }

    pub fn has_consumers(&self) -> bool {
        !self.consumers.lock().is_empty()
    }

    pub fn attach_sink(&mut self, sink: &AppSink) {
        if self
            .attached_sink
            .as_ref()
            .is_some_and(|current| current == sink)
        {
            return;
        }

        if let Some(old_sink) = self.attached_sink.take() {
            old_sink.set_callbacks(AppSinkCallbacks::builder().build());
        }

        Self::configure_callbacks(sink, self.consumers.clone());
        self.attached_sink = Some(sink.clone());
    }

    fn configure_callbacks(sink: &AppSink, consumers: Arc<Mutex<HashMap<String, AppSrc>>>) {
        let consumers_for_samples = consumers.clone();
        let consumers_for_eos = consumers.clone();

        sink.set_callbacks(
            AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let caps = sample.caps().map(|caps| caps.to_owned());
                    let Some(buffer) = sample.buffer() else {
                        return Ok(gst::FlowSuccess::Ok);
                    };

                    let snapshot = {
                        let guard = consumers_for_samples.lock();
                        guard
                            .iter()
                            .map(|(id, appsrc)| (id.clone(), appsrc.clone()))
                            .collect::<Vec<_>>()
                    };

                    if snapshot.is_empty() {
                        return Ok(gst::FlowSuccess::Ok);
                    }

                    let mut stale = Vec::new();
                    for (consumer_id, appsrc) in snapshot {
                        if let Some(caps) = caps.as_ref() {
                            appsrc.set_caps(Some(caps));
                        }

                        if appsrc.push_buffer(buffer.copy()).is_err() {
                            stale.push(consumer_id);
                        }
                    }

                    if !stale.is_empty() {
                        let mut guard = consumers_for_samples.lock();
                        for consumer_id in stale {
                            guard.remove(&consumer_id);
                        }
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .eos(move |_appsink| {
                    let snapshot = {
                        let guard = consumers_for_eos.lock();
                        guard.values().cloned().collect::<Vec<_>>()
                    };
                    for appsrc in snapshot {
                        let _ = appsrc.end_of_stream();
                    }
                })
                .build(),
        );
    }
}
