use async_compat::CompatExt;
use futures_lite::io::AsyncWriteExt;
use futures_lite::AsyncReadExt;
use lazy_static::lazy_static;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use simple_log::info;
use ssh_wrap::wezterm_ssh::SessionBuilder;
use std::time::Instant;
use tokio::task;
use wezterm_ssh::{Sftp, Utf8PathBuf};

const FILE_SIZE: usize = 1024 * 512;

lazy_static! {
    static ref FOO: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(FILE_SIZE)
        .map(char::from)
        .collect();
}

const TASK_NUM: usize = 10;
const SINGLE_TASK_FILE: usize = 2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_log::quick!();

    let _ = std::fs::remove_dir_all("./tmp/sftp/");
    std::fs::create_dir_all("./tmp/sftp/").unwrap();

    let session = SessionBuilder::new_with_pass("foo", "0.0.0.0", "123456", 2222)
        .disable_userknown_hosts_file()
        .connect_with_pass()
        .await
        .unwrap();

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
    info!("write all file finished {:?}", now.elapsed());

    let real_digest = sha256::digest(&*FOO);

    let entrys = sftp.read_dir("/sftp_upload/").await.unwrap();
    let now = Instant::now();

    let mut vec = vec![];
    for (path, _) in entrys {
        let handle = task::spawn(read(sftp.clone(), path));
        vec.push(handle);
    }

    assert_eq!(vec.len(), TASK_NUM * SINGLE_TASK_FILE);
    for handle in vec {
        assert_eq!(real_digest, sha256::digest(handle.await.unwrap()))
    }

    info!("read all file finished {:?}", now.elapsed());

    let entries = std::fs::read_dir("./tmp/sftp").unwrap();

    let mut number = 0;

    let now = Instant::now();
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
    info!("read local files finished {:?}", now.elapsed());

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

async fn read(sftp: Sftp, path: Utf8PathBuf) -> Vec<u8> {
    let mut file = sftp.open(path).await.unwrap();
    let mut contents = Vec::with_capacity(10);
    file.read_to_end(&mut contents).await.unwrap();
    contents
}
