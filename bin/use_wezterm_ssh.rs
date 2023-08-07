use async_compat::CompatExt;
use futures_lite::io::AsyncWriteExt;
use lazy_static::lazy_static;
use simple_log::info;
use ssh_wrap_rs::wezterm_ssh::SessionBuilder;
use std::time::Instant;
use tokio::task;
use wezterm_ssh::Sftp;

lazy_static! {
    static ref FOO: String = std::fs::read_to_string("./rand_data.dat").unwrap();
}

const TASK_NUM: usize = 300;
const SINGLE_TASK_FILE: usize = 2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_log::quick!();

    let _ = std::fs::remove_dir_all("./tmp/sftp/");
    std::fs::create_dir_all("./tmp/sftp/").unwrap();

    let builder = SessionBuilder {
        user: "foo".to_string(),
        host: "0.0.0.0".to_string(),
        pass: "123456".to_string(),
        port: 2222,
        identities_only: None,
        userknown_hosts_file: Some("/dev/null".to_string()),
        wezterm_ssh_verbose: None,
        wezterm_ssh_backend: Default::default(),
    };

    let session = builder.connect().await.unwrap();

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
