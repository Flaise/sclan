use std::sync::mpsc::Sender;
use std::io::Result as IOResult;
use tokio::sync::mpsc::Receiver as MReceiver;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use crate::network::{FromNet, show_error};

pub enum ToLog {
    LogStart,
    LogMessage(String),
}

async fn open_file(create: bool) -> IOResult<File> {
    OpenOptions::new()
        .append(true)
        .create(create)
        .open("./sclan.log")
        .await
}

async fn write(file: &mut File, bytes: &[u8]) -> IOResult<()> {
    file.write_all(bytes).await?;
    file.sync_data().await
}

pub async fn task_log(mut to_app: Sender<FromNet>, mut messages: MReceiver<ToLog>) {
    let to_app = &mut to_app;
    
    let mut prev = false;
    let mut ofile = open_file(false).await.ok();
    loop {
        if ofile.is_some() != prev {
            prev = ofile.is_some();
            
            if let Err(_) = to_app.send(FromNet::Logging(prev)) {
                return;
            }
        }
        
        let message = messages.recv().await;
        match message {
            None => return,
            Some(ToLog::LogStart) => {
                if ofile.is_some() {
                    continue;
                }
                match open_file(true).await {
                    Ok(file) => {
                        ofile = Some(file);
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
                        
                        if !show_error(to_app, format!("error: {:?}", error)) {
                            return;
                        }
                    }
                }
            }
        }
    }
}
