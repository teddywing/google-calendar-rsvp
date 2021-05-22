use base64;
use google_calendar3::api::{Event, EventAttendee};
use google_calendar3::CalendarHub;
use home;
use hyper;
use hyper_rustls;
use regex::Regex;
use tokio;
use yup_oauth2 as oauth2;

use std::env;
use std::fmt;
use std::fs;
use std::process;
use std::str;
use std::string;


#[derive(Debug)]
enum EventResponseStatus {
    Accepted,
    Declined,
    Tentative,
}

impl fmt::Display for EventResponseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventResponseStatus::Accepted => write!(f, "accepted"),
            EventResponseStatus::Declined => write!(f, "declined"),
            EventResponseStatus::Tentative => write!(f, "tentative"),
        }
    }
}


#[derive(Debug)]
struct Eid {
    event_id: String,
    email: Option<String>,
}

impl Eid {
    fn new(event_id: String, email: Option<String>) -> Self {
        Eid { event_id, email }
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mut action_opt: Option<EventResponseStatus> = None;
    let mut email = false;
    let mut event_ids = Vec::new();

    for arg in &args[1..] {
        match arg.as_ref() {
            "-y" | "--yes" =>
                action_opt = Some(EventResponseStatus::Accepted),
            "-n" | "--no" =>
                action_opt = Some(EventResponseStatus::Declined),
            "-m" | "--maybe" =>
                action_opt = Some(EventResponseStatus::Tentative),

            "--email" =>
                email = true,

            id =>
                event_ids.push(id),
        }
    }

    let action = match action_opt {
        Some(a) => a,
        None => {
            eprintln!("error: missing required action argument: --yes | --no | --maybe");

            process::exit(exitcode::USAGE);
        },
    };

    if event_ids.is_empty() {
        eprintln!("error: missing event ID argument");

        process::exit(exitcode::USAGE);
    }

    for event_id in &event_ids {
        rsvp(
            &event_id_from_base64(event_id),
            &action,
        ).await;
    }

    Ok(())
}

async fn rsvp(eid: &Eid, response: &EventResponseStatus) {
    let secret = secret_from_file();

    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
        .persist_tokens_to_disk(
            home::home_dir()
                .unwrap()
                .join(".google-service-cli/google-calendar-rsvp")
        )
        .build().await.unwrap();

    let hub = CalendarHub::new(
        hyper::Client::builder()
            .build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    let mut event = Event::default();
    let mut attendee = EventAttendee::default();

    if let Some(email) = &eid.email {
        attendee.email = Some(email.to_owned());
    } else {
        let result = hub.events()
            .get("primary", &eid.event_id)
            .doit()
            .await
            .unwrap();

        if let Some(attendees) = result.1.attendees {
            for a in &attendees {
                if let Some(is_me) = a.self_ {
                    if is_me {
                        attendee.email = a.email.clone();

                        break;
                    }
                }
            }
        }
    }

    attendee.response_status = Some(response.to_string());

    event.attendees = Some(vec![attendee]);

    let res = hub.events()
        .patch(event, "primary", &eid.event_id)
        .doit()
        .await
        .unwrap();

    dbg!(res);
}

fn secret_from_file() -> oauth2::ApplicationSecret {
    let f = fs::File::open(
        home::home_dir()
            .unwrap()
            .join(".google-service-cli/calendar3-secret.json"),
    ).unwrap();

    let console_secret: oauth2::ConsoleApplicationSecret = serde_json::from_reader(f).unwrap();

    match console_secret.installed {
        Some(secret) => secret,
        None => todo!(),
    }
}

fn event_id_from_base64(event_id: &str) -> Eid {
    // Base64-matching regex from Xuanyuanzhiyuan
    // (https://stackoverflow.com/users/1076906/xuanyuanzhiyuan) on Stack
    // Overflow:
    // https://stackoverflow.com/questions/8571501/how-to-check-whether-a-string-is-base64-encoded-or-not/8571649#8571649
    let re = Regex::new(
        "^([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{3}=|[A-Za-z0-9+/]{2}==)?$",
    ).unwrap();

    if !re.is_match(event_id) {
        return Eid::new(event_id.to_owned(), None);
    }

    let decoded = &base64::decode(event_id).unwrap();
    let id_email_pair = str::from_utf8(decoded).unwrap();
    let values = id_email_pair.split(" ").collect::<Vec<_>>();
    let id = values.first().unwrap().to_string();

    Eid::new(id, values.last().map(string::ToString::to_string))
}


#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_event_id_from_base64_event_id() {
        let expected = "1g4j1h67ndq7kddrb2bptp2cua_20210521T120000Z";

        let id = event_id_from_base64(expected);

        assert_eq!(expected, id);
    }

    #[test]
    fn test_event_id_from_base64_eid() {
        let expected = "1g4j1h67ndq7kddrb2bptp2cua";
        let encoded = base64::encode(format!("{} rory.mercury@example.com", expected));

        let id = event_id_from_base64(&encoded);

        assert_eq!(expected, id);
    }
}
