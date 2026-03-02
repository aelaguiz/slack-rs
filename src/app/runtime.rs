use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::app::action::Action;

#[derive(Debug)]
pub enum AppEvent {
    SlackConnected,
    SlackEventReceived,
    Action(Action),
    Fatal(anyhow::Error),
}

pub struct AppRuntime {
    rt: Option<Runtime>,
    rx: mpsc::Receiver<AppEvent>,
}

impl AppRuntime {
    pub fn new() -> anyhow::Result<(Self, mpsc::Sender<AppEvent>)> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let (tx, rx) = mpsc::channel(256);
        Ok((Self { rt: Some(rt), rx }, tx))
    }

    pub fn spawn<F>(&self, fut: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.rt.as_ref().expect("runtime is initialized").spawn(fut);
    }

    pub fn try_recv(&mut self) -> Option<AppEvent> {
        match self.rx.try_recv() {
            Ok(ev) => Some(ev),
            Err(mpsc::error::TryRecvError::Empty) => None,
            Err(mpsc::error::TryRecvError::Disconnected) => None,
        }
    }
}

impl Drop for AppRuntime {
    fn drop(&mut self) {
        // Avoid hanging on drop if a background task is still running (Socket Mode will be).
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}
