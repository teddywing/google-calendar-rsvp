use google_calendar3::api::{Event, EventAttendee};
use google_calendar3::{CalendarHub, Result, Error};
use home;
use hyper;
use hyper_rustls;
use tokio;
use yup_oauth2 as oauth2;

use std::fs;


#[tokio::main]
async fn main() {
    rsvp().await;
}

async fn rsvp() {
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

    let result = hub.events()
        .get("primary", "1g4j1h67ndq7kddrb2bptp2cua")
        .doit()
        .await
        .unwrap();

    let mut event = Event::default();
    let mut attendee = EventAttendee::default();

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

    attendee.response_status = Some("accepted".to_owned());

    event.attendees = Some(vec![attendee]);

    let res = hub.events()
        .patch(event, "primary", "1g4j1h67ndq7kddrb2bptp2cua")
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
