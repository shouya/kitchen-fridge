//! Calendar events (iCal `VEVENT` items)

use chrono::{DateTime, Utc};
use ical::property::Property;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::item::SyncStatus;
use crate::utils::random_url;

/// This struct currently does not support all-day events
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// The event URL
    url: Url,

    /// Persistent, globally unique identifier for the calendar component
    /// The [RFC](https://tools.ietf.org/html/rfc5545#page-117) recommends concatenating a timestamp with the server's domain name.
    /// UUID are even better so we'll generate them, but we have to support tasks from the server, that may have any arbitrary strings here.
    uid: String,

    /// SUMMARY
    name: String,

    /// DESCRIPTION
    description: Option<String>,

    sync_status: SyncStatus,

    /// The PRODID, as defined in iCal files
    ical_prod_id: String,

    creation_date: Option<DateTime<Utc>>,
    last_modified: DateTime<Utc>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,

    /// Extra parameters that have not been parsed from the iCal file (because they're not supported (yet) by this crate).
    /// They are needed to serialize this item into an equivalent iCal file
    extra_parameters: Vec<Property>,
}

impl Event {
    pub fn new(
        name: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        parent_calendar_url: &Url,
    ) -> Self {
        let new_url = random_url(parent_calendar_url);
        let new_sync_status = SyncStatus::NotSynced;
        let new_uid = Uuid::new_v4().to_hyphenated().to_string();
        let new_creation_date = Some(Utc::now());
        let new_last_modified = Utc::now();
        let new_description = None;
        let ical_prod_id = crate::ical::default_prod_id();
        let extra_parameters = Vec::new();
        Self::new_with_parameters(
            name,
            new_uid,
            new_url,
            new_description,
            new_sync_status,
            start,
            end,
            new_creation_date,
            new_last_modified,
            ical_prod_id,
            extra_parameters,
        )
    }

    pub fn new_with_parameters(
        name: String,
        uid: String,
        url: Url,
        description: Option<String>,
        sync_status: SyncStatus,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        creation_date: Option<DateTime<Utc>>,
        last_modified: DateTime<Utc>,
        ical_prod_id: String,
        extra_parameters: Vec<Property>,
    ) -> Self {
        Self {
            url,
            uid,
            name,
            description,
            sync_status,
            start,
            end,
            creation_date,
            last_modified,
            ical_prod_id,
            extra_parameters,
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ical_prod_id(&self) -> &str {
        &self.ical_prod_id
    }

    pub fn creation_date(&self) -> Option<&DateTime<Utc>> {
        self.creation_date.as_ref()
    }

    pub fn last_modified(&self) -> &DateTime<Utc> {
        &self.last_modified
    }

    pub fn sync_status(&self) -> &SyncStatus {
        &self.sync_status
    }
    pub fn set_sync_status(&mut self, new_status: SyncStatus) {
        self.sync_status = new_status;
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn has_same_observable_content_as(&self, _other: &Event) -> bool {
        unimplemented!();
    }
}
