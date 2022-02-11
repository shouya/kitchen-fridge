//! A module to parse ICal files

use std::error::Error;

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use ical::parser::ical::component::{IcalCalendar, IcalEvent, IcalTodo};
use ical::property::Property;
use url::Url;

use crate::item::SyncStatus;
use crate::task::CompletionStatus;
use crate::Event;
use crate::Item;
use crate::Task;

/// Parse an iCal file into the internal representation [`crate::Item`]
pub fn parse(
    content: &str,
    item_url: Url,
    sync_status: SyncStatus,
) -> Result<Item, Box<dyn Error>> {
    let mut reader = ical::IcalParser::new(content.as_bytes());
    let parsed_item = match reader.next() {
        None => return Err(format!("Invalid iCal data to parse for item {}", item_url).into()),
        Some(item) => match item {
            Err(err) => {
                return Err(
                    format!("Unable to parse iCal data for item {}: {}", item_url, err).into(),
                )
            }
            Ok(item) => item,
        },
    };

    let ical_prod_id = extract_ical_prod_id(&parsed_item)
        .map(|s| s.to_string())
        .unwrap_or_else(super::default_prod_id);

    let item = match assert_single_type(&parsed_item)? {
        CurrentType::Event(event) => {
            Item::Event(parse_event(event, item_url, sync_status, ical_prod_id)?)
        }
        CurrentType::Todo(todo) => {
            Item::Task(parse_task(todo, item_url, sync_status, ical_prod_id)?)
        }
    };

    // What to do with multiple items?
    if reader.next().map(|r| r.is_ok()) == Some(true) {
        return Err("Parsing multiple items are not supported".into());
    }

    Ok(item)
}

fn parse_task(
    todo: &IcalTodo,
    item_url: Url,
    sync_status: SyncStatus,
    ical_prod_id: String,
) -> Result<Task, Box<dyn Error>> {
    let mut name = None;
    let mut uid = None;
    let mut completed = false;
    let mut last_modified = None;
    let mut completion_date = None;
    let mut creation_date = None;
    let mut extra_parameters = Vec::new();

    for prop in &todo.properties {
        match prop.name.as_str() {
            "SUMMARY" => name = prop.value.clone(),
            "UID" => uid = prop.value.clone(),
            "DTSTAMP" => {
                // The property can be specified once, but is not mandatory
                // "This property specifies the date and time that the information associated with
                //  the calendar component was last revised in the calendar store."
                // "In the case of an iCalendar object that doesn't specify a "METHOD"
                //  property [e.g.: VTODO and VEVENT], this property is equivalent to the "LAST-MODIFIED" property".
                last_modified = parse_date_time_from_property(prop);
            }
            "LAST-MODIFIED" => {
                // The property can be specified once, but is not mandatory
                // "This property specifies the date and time that the information associated with
                //  the calendar component was last revised in the calendar store."
                // In practise, for VEVENT and VTODO, this is generally the same value as DTSTAMP.
                last_modified = parse_date_time_from_property(prop);
            }
            "COMPLETED" => {
                // The property can be specified once, but is not mandatory
                // "This property defines the date and time that a to-do was
                //  actually completed."
                completion_date = parse_date_time_from_property(prop)
            }
            "CREATED" => {
                // The property can be specified once, but is not mandatory
                creation_date = parse_date_time_from_property(prop)
            }
            "STATUS" => {
                // Possible values:
                //   "NEEDS-ACTION" ;Indicates to-do needs action.
                //   "COMPLETED"    ;Indicates to-do completed.
                //   "IN-PROCESS"   ;Indicates to-do in process of.
                //   "CANCELLED"    ;Indicates to-do was cancelled.
                if prop.value.as_ref().map(|s| s.as_str()) == Some("COMPLETED") {
                    completed = true;
                }
            }
            _ => {
                // This field is not supported. Let's store it anyway, so that we are able to re-create an identical iCal file
                extra_parameters.push(prop.clone());
            }
        }
    }
    let name = match name {
        Some(name) => name,
        None => return Err(format!("Missing name for item {}", item_url).into()),
    };
    let uid = match uid {
        Some(uid) => uid,
        None => return Err(format!("Missing UID for item {}", item_url).into()),
    };
    let last_modified = match last_modified {
        Some(dt) => dt,
        None => {
            return Err(format!(
                "Missing DTSTAMP for item {}, but this is required by RFC5545",
                item_url
            )
            .into())
        }
    };
    let completion_status = match completed {
        false => {
            if completion_date.is_some() {
                log::warn!("Task {:?} has an inconsistent content: its STATUS is not completed, yet it has a COMPLETED timestamp at {:?}", uid, completion_date);
            }
            CompletionStatus::Uncompleted
        }
        true => CompletionStatus::Completed(completion_date),
    };

    Ok(Task::new_with_parameters(
        name,
        uid,
        item_url,
        completion_status,
        sync_status,
        creation_date,
        last_modified,
        ical_prod_id,
        extra_parameters,
    ))
}

fn parse_event(
    event: &IcalEvent,
    item_url: Url,
    sync_status: SyncStatus,
    ical_prod_id: String,
) -> Result<Event, Box<dyn Error>> {
    let mut name = None;
    let mut description = None;
    let mut uid = None;
    let mut last_modified = None;
    let mut creation_date = None;
    let mut start = None;
    let mut end = None;
    let mut extra_parameters = Vec::new();

    for prop in &event.properties {
        match prop.name.as_str() {
            "SUMMARY" => name = prop.value.clone(),
            "DESCRIPTION" => description = prop.value.clone(),
            "UID" => uid = prop.value.clone(),
            "DTSTAMP" => {
                // The property can be specified once, but is not mandatory
                // "This property specifies the date and time that the information associated with
                //  the calendar component was last revised in the calendar store."
                // "In the case of an iCalendar object that doesn't specify a "METHOD"
                //  property [e.g.: VTODO and VEVENT], this property is equivalent to the "LAST-MODIFIED" property".
                last_modified = parse_date_time_from_property(prop);
            }
            "DTSTART" => {
                start = parse_date_time_from_property(prop);
            }
            "DTEND" => {
                end = parse_date_time_from_property(prop);
            }
            "LAST-MODIFIED" => {
                // The property can be specified once, but is not mandatory
                // "This property specifies the date and time that the information associated with
                //  the calendar component was last revised in the calendar store."
                // In practise, for VEVENT and VTODO, this is generally the same value as DTSTAMP.
                last_modified = parse_date_time_from_property(prop);
            }
            "CREATED" => {
                // The property can be specified once, but is not mandatory
                creation_date = parse_date_time_from_property(prop)
            }
            _ => {
                // This field is not supported. Let's store it anyway, so that we are able to re-create an identical iCal file
                extra_parameters.push(prop.clone());
            }
        }
    }
    let name = match name {
        Some(name) => name,
        None => return Err(format!("Missing name for item {}", item_url).into()),
    };
    let uid = match uid {
        Some(uid) => uid,
        None => return Err(format!("Missing UID for item {}", item_url).into()),
    };
    let last_modified = match last_modified {
        Some(dt) => dt,
        None => {
            return Err(format!(
                "Missing DTSTAMP for item {}, but this is required by RFC5545",
                item_url
            )
            .into())
        }
    };
    let start = start.ok_or_else(|| format!("Missing DTSTART for item {}", item_url))?;
    let end = end.ok_or_else(|| format!("Missing DTEND for item {}", item_url))?;

    Ok(Event::new_with_parameters(
        name,
        uid,
        item_url,
        description,
        sync_status,
        start,
        end,
        creation_date,
        last_modified,
        ical_prod_id,
        extra_parameters,
    ))
}

fn parse_date_time_from_property(property: &Property) -> Option<DateTime<Utc>> {
    use std::str::FromStr;

    let tzid: Option<&String> = property.params.as_ref().and_then(|params| {
        params
            .iter()
            .find_map(|(n, v)| (n == "TZID").then(|| ()).and_then(|_| v.iter().next()))
    });

    let s: &str = property.value.as_deref()?;
    if let Ok(t) = Utc.datetime_from_str(s, "%Y%m%dT%H%M%SZ") {
        return Some(t);
    }

    if let Some(tz) = tzid.and_then(|tz| Tz::from_str(tz).ok()) {
        if let Ok(t) = tz.datetime_from_str(s, "%Y%m%dT%H%M%S") {
            return Some(t.with_timezone(&Utc));
        }
    }

    Utc.datetime_from_str(s, "%Y%m%dT%H%M%S").ok()
}

fn extract_ical_prod_id(item: &IcalCalendar) -> Option<&str> {
    for prop in &item.properties {
        if &prop.name == "PRODID" {
            return prop.value.as_ref().map(|s| s.as_str());
        }
    }
    None
}

enum CurrentType<'a> {
    Event(&'a IcalEvent),
    Todo(&'a IcalTodo),
}

fn assert_single_type<'a>(item: &'a IcalCalendar) -> Result<CurrentType<'a>, Box<dyn Error>> {
    let n_events = item.events.len();
    let n_todos = item.todos.len();
    let n_journals = item.journals.len();

    if n_events == 1 {
        if n_todos != 0 || n_journals != 0 {
            return Err("Only a single TODO or a single EVENT is supported".into());
        } else {
            return Ok(CurrentType::Event(&item.events[0]));
        }
    }

    if n_todos == 1 {
        if n_events != 0 || n_journals != 0 {
            return Err("Only a single TODO or a single EVENT is supported".into());
        } else {
            return Ok(CurrentType::Todo(&item.todos[0]));
        }
    }

    return Err("Only a single TODO or a single EVENT is supported".into());
}

#[cfg(test)]
mod test {
    const EXAMPLE_ICAL: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:0633de27-8c32-42be-bcb8-63bc879c6185@some-domain.com
CREATED:20210321T001600
LAST-MODIFIED:20210321T001600
DTSTAMP:20210321T001600
SUMMARY:Do not forget to do this
END:VTODO
END:VCALENDAR
"#;

    const EXAMPLE_ICAL_COMPLETED: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:19960401T080045Z-4000F192713-0052@example.com
CREATED:20210321T001600
LAST-MODIFIED:20210402T081557
DTSTAMP:20210402T081557
SUMMARY:Clean up your room or Mom will be angry
PERCENT-COMPLETE:100
COMPLETED:20210402T081557
STATUS:COMPLETED
END:VTODO
END:VCALENDAR
"#;

    const EXAMPLE_ICAL_COMPLETED_WITHOUT_A_COMPLETION_DATE: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:19960401T080045Z-4000F192713-0052@example.com
CREATED:20210321T001600
LAST-MODIFIED:20210402T081557
DTSTAMP:20210402T081557
SUMMARY:Clean up your room or Mom will be angry
STATUS:COMPLETED
END:VTODO
END:VCALENDAR
"#;

    const EXAMPLE_MULTIPLE_ICAL: &str = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Nextcloud Tasks v0.13.6
BEGIN:VTODO
UID:0633de27-8c32-42be-bcb8-63bc879c6185
CREATED:20210321T001600
LAST-MODIFIED:20210321T001600
DTSTAMP:20210321T001600
SUMMARY:Call Mom
END:VTODO
END:VCALENDAR
BEGIN:VCALENDAR
BEGIN:VTODO
UID:0633de27-8c32-42be-bcb8-63bc879c6185
CREATED:20210321T001600
LAST-MODIFIED:20210321T001600
DTSTAMP:20210321T001600
SUMMARY:Buy a gift for Mom
END:VTODO
END:VCALENDAR
"#;

    use super::*;
    use crate::item::VersionTag;

    #[test]
    fn test_ical_parsing() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(EXAMPLE_ICAL, item_url.clone(), sync_status.clone()).unwrap();
        let task = item.unwrap_task();

        assert_eq!(task.name(), "Do not forget to do this");
        assert_eq!(task.url(), &item_url);
        assert_eq!(
            task.uid(),
            "0633de27-8c32-42be-bcb8-63bc879c6185@some-domain.com"
        );
        assert_eq!(task.completed(), false);
        assert_eq!(task.completion_status(), &CompletionStatus::Uncompleted);
        assert_eq!(task.sync_status(), &sync_status);
        assert_eq!(
            task.last_modified(),
            &Utc.ymd(2021, 3, 21).and_hms(0, 16, 0)
        );
    }

    #[test]
    fn test_completed_ical_parsing() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(
            EXAMPLE_ICAL_COMPLETED,
            item_url.clone(),
            sync_status.clone(),
        )
        .unwrap();
        let task = item.unwrap_task();

        assert_eq!(task.completed(), true);
        assert_eq!(
            task.completion_status(),
            &CompletionStatus::Completed(Some(Utc.ymd(2021, 4, 2).and_hms(8, 15, 57)))
        );
    }

    #[test]
    fn test_completed_without_date_ical_parsing() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(
            EXAMPLE_ICAL_COMPLETED_WITHOUT_A_COMPLETION_DATE,
            item_url.clone(),
            sync_status.clone(),
        )
        .unwrap();
        let task = item.unwrap_task();

        assert_eq!(task.completed(), true);
        assert_eq!(task.completion_status(), &CompletionStatus::Completed(None));
    }

    #[test]
    fn test_multiple_items_in_ical() {
        let version_tag = VersionTag::from(String::from("test-tag"));
        let sync_status = SyncStatus::Synced(version_tag);
        let item_url: Url = "http://some.id/for/testing".parse().unwrap();

        let item = parse(EXAMPLE_MULTIPLE_ICAL, item_url.clone(), sync_status.clone());
        assert!(item.is_err());
    }
}
