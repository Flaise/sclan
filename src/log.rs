use std::sync::mpsc::Sender;
use std::io::Result as IOResult;
use tokio::sync::mpsc::{Receiver as MReceiver, Sender as MSender};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use crate::network::{FromNet, ToNet, show_error};

pub enum ToLog {
    LogStart,
    LogMessage(String),
}

async fn open_file() -> IOResult<File> {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open("./sclan.log")
        .await
}

async fn write(file: &mut File, bytes: &[u8]) -> IOResult<()> {
    file.write_all(bytes).await?;
    file.sync_data().await
}

async fn task_logging(to_app: &mut Sender<FromNet>, mut messages: MReceiver<ToLog>) {
    let mut ofile = open_file().await.ok();
    if ofile.is_some() {
        if let Err(_) = to_app.send(FromNet::Logging(true)) {
            return;
        }
    }
    loop {
        let message = messages.recv().await;
        match message {
            None => return,
            Some(ToLog::LogStart) => {
                if ofile.is_some() {
                    continue;
                }
                match open_file().await {
                    Ok(file) => {
                        ofile = Some(file);
                        if let Err(_) = to_app.send(FromNet::Logging(true)) {
                            return;
                        }
                    }
                    Err(error) => {
                        if let Err(_) = to_app.send(FromNet::Logging(false)) {
                            return;
                        }
                        if !show_error(to_app, format!("error: {:?}", error)) {
                            return;
                        }
                    }
                }
            }
            Some(ToLog::LogMessage(content)) => {
                if let Some(ref mut file) = ofile {
                    if let Err(error) = write(file, content.as_bytes()).await {
                        ofile = None;
                        
                        if let Err(_) = to_app.send(FromNet::Logging(false)) {
                            return;
                        }
                        if !show_error(to_app, format!("error: {:?}", error)) {
                            return;
                        }
                    }
                }
            }
        }
    }
}
