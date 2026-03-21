use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use xconnect_protocol::{DevicePlatform, SessionState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserRecord {
    pub user_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub device_id: Uuid,
    pub account_id: Uuid,
    pub device_name: String,
    pub platform: DevicePlatform,
    pub trusted: bool,
    pub unattended_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: Uuid,
    pub account_id: Uuid,
    pub caller_device_id: Uuid,
    pub target_device_id: Uuid,
    pub unattended: bool,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Default)]
pub struct InMemoryDb {
    users_by_id: DashMap<Uuid, UserRecord>,
    user_id_by_email: DashMap<String, Uuid>,
    devices: DashMap<Uuid, DeviceRecord>,
    sessions: DashMap<Uuid, SessionRecord>,
}

impl InMemoryDb {
    pub fn create_user(&self, email: String, password_hash: String) -> Option<UserRecord> {
        if self.user_id_by_email.contains_key(&email) {
            return None;
        }

        let user = UserRecord {
            user_id: Uuid::new_v4(),
            email: email.clone(),
            password_hash,
            created_at: Utc::now(),
        };

        self.user_id_by_email.insert(email, user.user_id);
        self.users_by_id.insert(user.user_id, user.clone());
        Some(user)
    }

    pub fn find_user_by_email(&self, email: &str) -> Option<UserRecord> {
        let user_id = self.user_id_by_email.get(email)?.value().to_owned();
        self.users_by_id.get(&user_id).map(|row| row.clone())
    }

    pub fn register_device(
        &self,
        account_id: Uuid,
        device_name: String,
        platform: DevicePlatform,
        trusted: bool,
        unattended_enabled: bool,
    ) -> DeviceRecord {
        let row = DeviceRecord {
            device_id: Uuid::new_v4(),
            account_id,
            device_name,
            platform,
            trusted,
            unattended_enabled,
            created_at: Utc::now(),
        };
        self.devices.insert(row.device_id, row.clone());
        row
    }

    pub fn list_devices_by_account(&self, account_id: Uuid) -> Vec<DeviceRecord> {
        self.devices
            .iter()
            .filter(|row| row.account_id == account_id)
            .map(|row| row.clone())
            .collect()
    }

    pub fn set_device_trust(
        &self,
        account_id: Uuid,
        device_id: Uuid,
        trusted: bool,
    ) -> Option<DeviceRecord> {
        let mut entry = self.devices.get_mut(&device_id)?;
        if entry.account_id != account_id {
            return None;
        }
        entry.trusted = trusted;
        Some(entry.clone())
    }

    pub fn delete_device(&self, account_id: Uuid, device_id: Uuid) -> bool {
        if let Some(device) = self.devices.get(&device_id) {
            if device.account_id != account_id {
                return false;
            }
        } else {
            return false;
        }
        self.devices.remove(&device_id).is_some()
    }

    pub fn get_device(&self, device_id: Uuid) -> Option<DeviceRecord> {
        self.devices.get(&device_id).map(|row| row.clone())
    }

    pub fn create_session(
        &self,
        account_id: Uuid,
        caller_device_id: Uuid,
        target_device_id: Uuid,
        unattended: bool,
    ) -> SessionRecord {
        let row = SessionRecord {
            session_id: Uuid::new_v4(),
            account_id,
            caller_device_id,
            target_device_id,
            unattended,
            state: SessionState::Requested,
            created_at: Utc::now(),
        };
        self.sessions.insert(row.session_id, row.clone());
        row
    }

    pub fn get_session(&self, session_id: Uuid) -> Option<SessionRecord> {
        self.sessions.get(&session_id).map(|row| row.clone())
    }

    pub fn set_session_state(
        &self,
        account_id: Uuid,
        session_id: Uuid,
        new_state: SessionState,
    ) -> Option<SessionRecord> {
        let mut entry = self.sessions.get_mut(&session_id)?;
        if entry.account_id != account_id {
            return None;
        }
        entry.state = new_state;
        Some(entry.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_user_rejects_duplicate_email() {
        let db = InMemoryDb::default();
        let first = db.create_user("alice@example.com".to_string(), "h1".to_string());
        let second = db.create_user("alice@example.com".to_string(), "h2".to_string());
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn set_device_trust_only_owner() {
        let db = InMemoryDb::default();
        let account_a = Uuid::new_v4();
        let account_b = Uuid::new_v4();

        let device = db.register_device(
            account_a,
            "A".to_string(),
            DevicePlatform::Windows,
            false,
            false,
        );

        let denied = db.set_device_trust(account_b, device.device_id, true);
        assert!(denied.is_none());

        let allowed = db.set_device_trust(account_a, device.device_id, true);
        assert!(allowed.is_some());
        assert!(allowed.expect("updated").trusted);
    }
}
