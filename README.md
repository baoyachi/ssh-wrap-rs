# ssh-bench-rs
ssh performance bench 

#### run bin use_wezterm_ssh 
* step 1: run docker-compose: docker-compose up -d
```bash
➜ docker-compose up -d
[+] Running 3/3
 ✔ Network ssh-bench-rs_default                                                                                                                        Created                                                                                   0.0s 
 ✔ Container ssh-bench-rs-sftp-1                                                                                                                       Started                                                                                   0.3s 
 ! sftp The requested image's platform (linux/amd64) does not match the detected host platform (linux/arm64/v8) and no specific platform was requested                                                                                           0.0s 
➜  ssh-bench-rs git:(main) 
```

* step 2: cargo run run bin use_wezterm_ssh
```bash
➜ cargo run --release --bin use_wezterm_ssh
```
