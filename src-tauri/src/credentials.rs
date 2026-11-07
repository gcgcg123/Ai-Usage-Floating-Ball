use keyring::Entry;

const CREDENTIAL_SERVICE: &str = "com.personal.aiusage.liquidball";

#[derive(Clone, Default)]
pub struct CredentialStore;

impl CredentialStore {
    pub fn set(&self, profile_id: &str, api_key: &str) -> Result<(), String> {
        let entry =
            Entry::new(CREDENTIAL_SERVICE, profile_id).map_err(|error| error.to_string())?;
        entry
            .set_password(api_key)
            .map_err(|error| error.to_string())
    }

    pub fn get(&self, profile_id: &str) -> Result<Option<String>, String> {
        let entry =
            Entry::new(CREDENTIAL_SERVICE, profile_id).map_err(|error| error.to_string())?;
        match entry.get_password() {
            Ok(api_key) => Ok(Some(api_key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn delete(&self, profile_id: &str) -> Result<(), String> {
        let entry =
            Entry::new(CREDENTIAL_SERVICE, profile_id).map_err(|error| error.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Windows Credential Manager"]
    fn windows_credential_round_trip() {
        let store = CredentialStore;
        let profile_id = format!("test-{}", uuid::Uuid::new_v4());
        store.set(&profile_id, "test-secret").unwrap();
        assert_eq!(
            store.get(&profile_id).unwrap().as_deref(),
            Some("test-secret")
        );
        store.delete(&profile_id).unwrap();
        assert!(store.get(&profile_id).unwrap().is_none());
    }
}
