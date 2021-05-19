use google_calendar3::{CalendarHub, Result, Error};
use home;
use hyper;
use hyper_rustls;
use tokio;
use yup_oauth2 as oauth2;

use std::fs;


#[tokio::main]
async fn main() {
    // let secret: oauth2::ApplicationSecret = Default::default();
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
        .await;

    match result {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
             Error::HttpError(_)
            |Error::Io(_)
            |Error::MissingAPIKey
            |Error::MissingToken(_)
            |Error::Cancelled
            |Error::UploadSizeLimitExceeded(_, _)
            |Error::Failure(_)
            |Error::BadRequest(_)
            |Error::FieldClash(_)
            |Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => {
            println!("Success: {:?}", res);

            dbg!(&res.1.attendees);

            let event = res.1;

            if let Some(ref original_attendees) = event.attendees {
                let mut attendees = original_attendees.clone();

                for mut attendee in &mut attendees {
                    if let Some(is_me) = attendee.self_ {
                        if is_me {
                            attendee.response_status = Some("accepted".to_owned());

                            break;
                        }
                    }
                }

                let patched_attendees = serde_json::to_string(&attendees)
                    .unwrap();

                dbg!(&patched_attendees);

                let res = hub.events()
                    .patch(event, "primary", "1g4j1h67ndq7kddrb2bptp2cua")
                    .param("attendees", &patched_attendees)
                    .doit()
                    .await
                    .unwrap();

                dbg!(res);
            }
        },
    }
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
