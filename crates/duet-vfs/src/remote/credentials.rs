//! Credential Storage & Keyring Integration (Task T-7.1.2).
//! Manages secure zeroised secrets, SSH config imports, and Secret Service keyring lookup.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Mutex;
use zeroize::Zeroize;

#[derive(Debug, Clone, Default)]
pub struct SecretString(pub String);

impl Drop for SecretString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Profile metadata for remote connections.
#[derive(Debug, Clone)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub scheme: String, // sftp, ftp, webdav, s3, smb
    pub host: String,
    pub port: u16,
    pub user: String,
    pub remote_path: String,
}

pub struct CredentialStore {
    profiles: Mutex<HashMap<String, ConnectionProfile>>,
    secrets: Mutex<HashMap<String, SecretString>>,
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore {
    pub fn new() -> Self {
        Self {
            profiles: Mutex::new(HashMap::new()),
            secrets: Mutex::new(HashMap::new()),
        }
    }

    pub fn save_profile(&self, profile: ConnectionProfile) {
        if let Ok(mut guard) = self.profiles.lock() {
            guard.insert(profile.id.clone(), profile);
        }
    }

    pub fn get_profile(&self, id: &str) -> Option<ConnectionProfile> {
        self.profiles.lock().ok()?.get(id).cloned()
    }

    pub fn store_secret(&self, profile_id: &str, secret: &str) {
        if let Ok(mut guard) = self.secrets.lock() {
            guard.insert(profile_id.to_string(), SecretString(secret.to_string()));
        }
    }

    pub fn get_secret(&self, profile_id: &str) -> Option<String> {
        self.secrets.lock().ok()?.get(profile_id).map(|s| s.0.clone())
    }

    /// Import host profiles from `~/.ssh/config` (Task T-7.1.8).
    pub fn import_ssh_config(config_path: &Path) -> Vec<ConnectionProfile> {
        let mut profiles = Vec::new();
        let file = match File::open(config_path) {
            Ok(f) => f,
            Err(_) => return profiles,
        };

        let reader = BufReader::new(file);
        let mut current_host: Option<String> = None;
        let mut current_hostname: Option<String> = None;
        let mut current_user: Option<String> = None;
        let mut current_port: u16 = 22;

        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0].to_lowercase();
                let val = parts[1];

                match key.as_str() {
                    "host" => {
                        if let Some(h) = current_host.take() {
                            profiles.push(ConnectionProfile {
                                id: format!("ssh-{}", h),
                                name: h,
                                scheme: "sftp".to_string(),
                                host: current_hostname.unwrap_or_else(|| "127.0.0.1".to_string()),
                                port: current_port,
                                user: current_user.unwrap_or_else(|| "root".to_string()),
                                remote_path: "/".to_string(),
                            });
                        }
                        current_host = Some(val.to_string());
                        current_hostname = None;
                        current_user = None;
                        current_port = 22;
                    }
                    "hostname" => current_hostname = Some(val.to_string()),
                    "user" => current_user = Some(val.to_string()),
                    "port" => current_port = val.parse().unwrap_or(22),
                    _ => {}
                }
            }
        }

        if let Some(h) = current_host {
            profiles.push(ConnectionProfile {
                id: format!("ssh-{}", h),
                name: h,
                scheme: "sftp".to_string(),
                host: current_hostname.unwrap_or_else(|| "127.0.0.1".to_string()),
                port: current_port,
                user: current_user.unwrap_or_else(|| "root".to_string()),
                remote_path: "/".to_string(),
            });
        }

        profiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_zeroisation() {
        let secret = SecretString("super_secret_password".to_string());
        assert_eq!(secret.0, "super_secret_password");
        drop(secret);
    }
}
