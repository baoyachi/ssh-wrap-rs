use anyhow::bail;
use async_compat::CompatExt;
use std::string::ToString;
use wezterm_ssh::{Config, ConfigMap, Session, SessionEvent};

pub struct SessionBuilder {
    pub user: String,
    pub host: String,
    pub pass: String,
    pub port: u16,
    pub identities_only: Option<bool>,
    pub userknown_hosts_file: Option<String>,
    pub wezterm_ssh_verbose: Option<bool>,
    pub wezterm_ssh_backend: SshBackend,
}

#[derive(Display, Debug, Default)]
pub enum SshBackend {
    #[strum(serialize = "libssh")]
    Libssh,
    #[strum(serialize = "ssh2")]
    #[default]
    Ssh2,
}

impl SessionBuilder {
    pub fn new_with_pass<U, H, P>(user: U, host: H, pass: P, port: u16) -> Self
    where
        U: AsRef<str>,
        H: AsRef<str>,
        P: AsRef<str>,
    {
        Self {
            user: user.as_ref().to_string(),
            host: host.as_ref().to_string(),
            pass: pass.as_ref().to_string(),
            port,
            identities_only: None,
            userknown_hosts_file: None,
            wezterm_ssh_verbose: None,
            wezterm_ssh_backend: Default::default(),
        }
    }

    fn identities_only(&self) -> &str {
        if let Some(i) = self.identities_only {
            if i {
                return "yes";
            }
        }
        "no"
    }

    pub fn disable_userknown_hosts_file(&mut self) -> &mut Self {
        self.userknown_hosts_file = Some("/dev/null".to_string());
        self
    }

    fn wezterm_ssh_verbose(&self) -> &str {
        if let Some(i) = self.wezterm_ssh_verbose {
            if i {
                return "true";
            }
        }
        "false"
    }

    fn configmap(&self) -> ConfigMap {
        let config = Config::new();
        let mut config = config.for_host(&self.host);
        config.insert("port".to_string(), self.port.to_string());
        config.insert("user".to_string(), self.user.to_string());
        config.insert(
            "identitiesonly".to_string(),
            self.identities_only().to_string(),
        );
        config.insert(
            "wezterm_ssh_verbose".to_string(),
            self.wezterm_ssh_verbose().to_string(),
        );
        config.insert(
            "wezterm_ssh_backend".to_string(),
            self.wezterm_ssh_backend.to_string(),
        );

        if let Some(path) = &self.userknown_hosts_file {
            config.insert("userknownhostsfile".to_string(), path.to_string());
        }

        config
    }

    pub async fn connect_with_pass(&self) -> anyhow::Result<Session> {
        let (session, events) = Session::connect(self.configmap())?;
        while let Ok(event) = events.recv().await {
            match event {
                SessionEvent::Banner(banner) => {
                    info!("SessionEvent Banner:{:?}", banner);
                }
                SessionEvent::HostVerify(verify) => {
                    info!("SessionEvent HostVerify:{:?}", verify);
                    verify.answer(true).compat().await?;
                }
                SessionEvent::Authenticate(auth) => {
                    info!("SessionEvent Authenticate:{:?}", auth);
                    auth.answer(vec![self.pass.to_string()]).compat().await?;
                }
                SessionEvent::Error(err) => {
                    error!("SessionEvent Error:{:?}", err);
                    bail!(err)
                }
                SessionEvent::Authenticated => {
                    info!("SessionEvent Authenticated");
                    break;
                }
            }
        }
        Ok(session)
    }
}
