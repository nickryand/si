use std::time::Duration;

use si_data_nats::async_nats::jetstream;

pub const NATS_HEADER_DB_NAME: &str = "X-DB-NAME";
pub const NATS_HEADER_KEY: &str = "X-KEY";
pub const NATS_HEADER_INSTANCE_ID: &str = "X-INSTANCE-ID";

const NATS_EVENTS_STREAM_NAME: &str = "LAYERDB_EVENTS";

// Stream that covers messages across the following subjects:
// ```
// si.layerdb.events.$workspace_pk.$change_set_pk.$table_name.$event_kind
// ```
const NATS_EVENT_STREAM_SUBJECTS: &[&str] = &["si.layerdb.events.*.*.*.*"];

/// Returns a Jetstream Stream and creates it if it doesn't yet exist.
pub async fn layerdb_events_stream(
    context: &jetstream::Context,
    prefix: Option<&str>,
) -> Result<jetstream::stream::Stream, jetstream::context::CreateStreamError> {
    let subjects: Vec<_> = NATS_EVENT_STREAM_SUBJECTS
        .iter()
        .map(|suffix| subject::nats_subject(prefix, suffix).to_string())
        .collect();

    let stream = context
        .get_or_create_stream(jetstream::stream::Config {
            name: nats_stream_name(prefix, NATS_EVENTS_STREAM_NAME),
            description: Some("Layerdb events".to_owned()),
            subjects,
            retention: jetstream::stream::RetentionPolicy::Limits,
            discard: jetstream::stream::DiscardPolicy::Old,
            // TODO(fnichol): this likely needs tuning
            max_age: Duration::from_secs(60 * 60 * 6),
            ..Default::default()
        })
        .await?;

    Ok(stream)
}

fn nats_stream_name(prefix: Option<&str>, suffix: impl AsRef<str>) -> String {
    let suffix = suffix.as_ref();

    match prefix {
        Some(prefix) => {
            let mut s = String::with_capacity(prefix.len() + 1 + suffix.len());
            s.push_str(prefix);
            s.push('_');
            s.push_str(suffix);
            s
        }
        None => suffix.to_owned(),
    }
}

pub mod subject {
    use si_data_nats::Subject;

    use crate::event::LayeredEvent;

    const EVENTS_PREFIX: &str = "si.layerdb.events";

    pub fn for_event(prefix: Option<&str>, event: &LayeredEvent) -> Subject {
        // Cuts down on the amount of `String` allocations dealing with Ulids
        let mut buf = [0; ulid::ULID_LEN];

        // A string with enough capacity to avoid multiple reallocations
        let mut suffix = String::with_capacity(
            EVENTS_PREFIX.len() + (2 * ulid::ULID_LEN) + event.payload.db_name.len() + 4,
        );
        suffix.push_str(EVENTS_PREFIX);
        suffix.push('.');
        suffix.push_str(event.metadata.tenancy.workspace_pk.array_to_str(&mut buf));
        suffix.push('.');
        suffix.push_str(event.metadata.tenancy.change_set_pk.array_to_str(&mut buf));
        suffix.push('.');
        suffix.push_str(&event.payload.db_name);
        suffix.push('.');
        suffix.push_str(event.event_kind.as_ref());

        nats_subject(prefix, suffix)
    }

    pub(crate) fn nats_subject(prefix: Option<&str>, suffix: impl AsRef<str>) -> Subject {
        let suffix = suffix.as_ref();

        match prefix {
            Some(prefix) => {
                let mut s = String::with_capacity(prefix.len() + 1 + suffix.len());
                s.push_str(prefix);
                s.push('.');
                s.push_str(suffix);

                Subject::from(s)
            }
            None => Subject::from(suffix),
        }
    }
}
