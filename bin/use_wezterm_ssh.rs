use async_compat::CompatExt;
use futures_lite::io::AsyncWriteExt;
use lazy_static::lazy_static;
use simple_log::info;
use simple_log::log::warn;
use std::time::Instant;
use tokio::task;
use wezterm_ssh::{Config, Session, SessionEvent, Sftp};

lazy_static! {
    static ref FOO: String = std::fs::read_to_string("./rand_data.dat").unwrap();
}

const TASK_NUM: usize = 60;
const SINGLE_TASK_FILE: usize = 10;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_log::quick!();

    let _ = std::fs::remove_dir_all("./tmp/sftp/");
    std::fs::create_dir_all("./tmp/sftp/").unwrap();

    let config = Config::new();
    let mut config = config.for_host("0.0.0.0");
    config.insert("port".to_string(), "2222".to_string());
    config.insert("user".to_string(), "foo".to_string());
    config.insert("identitiesonly".to_string(), "no".to_string());
    config.insert("wezterm_ssh_verbose".to_string(), "false".to_string());
    config.insert("wezterm_ssh_backend".to_string(), "ssh2".to_string());

    let (session, events) = Session::connect(config).unwrap();

    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                SessionEvent::Banner(banner) => {
                    info!("ssh banner: {banner:?}");
                }
                SessionEvent::HostVerify(verify) => {
                    info!("[local] on_verify_host({:?})", verify);
                    verify.answer(true).compat().await.unwrap();
                }
                SessionEvent::Authenticate(auth) => {
                    info!("Authenticate:{:?}", auth);
                    auth.answer(vec!["123456".to_string()])
                        .compat()
                        .await
                        .unwrap();
                }
                SessionEvent::Error(err) => {
                    warn!("[local] on_error({err})");
                }
                SessionEvent::Authenticated => {
                    warn!("ssh authenticated");
                    break;
                }
            }
        }
    });

    let sftp = session.sftp();

    let now = Instant::now();
    let mut vec = vec![];
    for i in 0..TASK_NUM {
        let handle = task::spawn(write(sftp.clone(), i));
        vec.push(handle);
    }
    for handle in vec {
        handle.await.unwrap()
    }
    info!("{:?}", now.elapsed());

    let real_digest = sha256::digest(&*FOO);

    let entries = std::fs::read_dir("./tmp/sftp")?;

    let mut number = 0;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();

        let file_name = format!("./tmp/sftp/{}", file_name.to_string_lossy());

        if entry.file_type()?.is_file() {
            let digest = sha256::try_digest(file_name).unwrap();
            assert_eq!(digest, real_digest);
            number += 1;
        }
    }

    assert_eq!(number, TASK_NUM * SINGLE_TASK_FILE);

    Ok(())
}

async fn write(sftp: Sftp, task_index: usize) {
    let data = FOO.as_bytes();

    for i in 0..SINGLE_TASK_FILE {
        let file_name = format!("/sftp_upload/file_{}_{}", task_index, i);

        let mut file = sftp.create(&file_name).compat().await.unwrap();

        let file_id = format!("{:?}", file);
        info!("get file_id: {},file_name:{}", file_id, file_name);

        file.write_all(data.as_ref()).compat().await.unwrap();
        file.flush().await.unwrap();
        info!("end drop file,file_id: {},file_name:{}", file_id, file_name);
    }
}
