// Copyright (c) 2021  Teddy Wing
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use anyhow::{self, Context};
use base64;
use chrono::DateTime;
use google_calendar3::api::{Event, EventAttendee, Scope};
use google_calendar3::CalendarHub;
use hyper;
use hyper_rustls;
use mailparse;
use regex::Regex;
use tokio;
use xdg;
use yup_oauth2 as oauth2;

use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::process;
use std::str;


/// Event RSVP response status.
#[derive(Debug)]
enum EventResponseStatus {
    Accepted,
    Declined,
    Tentative,
}

impl fmt::Display for EventResponseStatus {
    /// Translate response status to the equivalent string required by the
    /// Google Calendar API.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventResponseStatus::Accepted => write!(f, "accepted"),
            EventResponseStatus::Declined => write!(f, "declined"),
            EventResponseStatus::Tentative => write!(f, "tentative"),
        }
    }
}


#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(e) => {
            eprint!("error");

            for cause in e.chain() {
                eprint!(": {}", cause);
            }

            eprintln!();

            process::exit(exitcode::SOFTWARE);
        },
    }
}

fn print_usage() {
    print!(r#"
usage: google-calendar-rsvp [options] [-y | -n | -m] [--email] [event_ids...]

Options:
    -y, --yes          rsvp with "yes"
    -n, --no           rsvp with "no"
    -m, --maybe        rsvp with "maybe"

        --email        read a Google Calendar invitation email from stdin

    -v, --verbose      enable verbose output
    -h, --help         print this help menu
    -V, --version      show the program version
"#);
}

async fn run() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut action_opt: Option<EventResponseStatus> = None;
    let mut should_read_email = false;

    #[allow(unused_assignments)]
    let mut email_eid = String::new();

    let mut event_ids = Vec::new();
    let mut is_verbose = false;

    for arg in &args[1..] {
        match arg.as_ref() {
            "-y" | "--yes" =>
                action_opt = Some(EventResponseStatus::Accepted),
            "-n" | "--no" =>
                action_opt = Some(EventResponseStatus::Declined),
            "-m" | "--maybe" =>
                action_opt = Some(EventResponseStatus::Tentative),

            "--email" =>
                should_read_email = true,

            "-v" | "--verbose" =>
                is_verbose = true,

            "-h" | "--help" => {
                print_usage();
                process::exit(exitcode::USAGE);
            },

            "-V" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                process::exit(exitcode::OK);
            },

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

    if should_read_email {
        let mut stdin = io::stdin();
        let mut email_input: Vec<u8> = Vec::new();
        stdin.read_to_end(&mut email_input)
            .context("unable to read standard input")?;

        email_eid = eid_from_email(&email_input)?;

        event_ids.push(&email_eid);
    }

    if event_ids.is_empty() {
        eprintln!("error: missing event ID argument");

        process::exit(exitcode::USAGE);
    }

    for event_id in &event_ids {
        let event = rsvp(
            &event_id_from_base64(event_id)?,
            &action,
        ).await?;

        if is_verbose {
            print_event(&event)?;
        }
    }

    Ok(())
}

/// RSVP to `event_id` with the status `response`.
async fn rsvp(event_id: &str, response: &EventResponseStatus) -> anyhow::Result<Event> {
    let secret = secret_from_file()?;

    let xdg_dirs = xdg::BaseDirectories::with_prefix("google-calendar-rsvp")
        .context("can't get XDG base directory")?;

    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
        .persist_tokens_to_disk(xdg_dirs.get_data_home().join("token.json"))
        .build()
        .await
        .context("authentication failed")?;

    let hub = CalendarHub::new(
        hyper::Client::builder()
            .build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    let get_response = hub.events()
        .get("primary", event_id)

        // Request read-write access to events so that we don't ask for
        // authorization a second time on the subsequent Event.patch call.
        .add_scope(Scope::Event)
        .doit()
        .await
        .with_context(|| format!("unable to get event '{}'", event_id))?;

    let mut event = Event::default();
    let mut attendee = EventAttendee::default();

    if let Some(attendees) = get_response.1.attendees {
        for a in &attendees {
            if let Some(is_me) = a.self_ {
                if is_me {
                    attendee.email = a.email.clone();

                    break;
                }
            }
        }
    }

    attendee.response_status = Some(response.to_string());

    event.attendees = Some(vec![attendee]);

    let rsvp_response = hub.events()
        .patch(event, "primary", event_id)

        // The default scope is Scope::Full.
        .add_scope(Scope::Event)
        .doit()
        .await
        .with_context(|| format!("unable to update event '{}'", event_id))?;

    Ok(rsvp_response.1)
}

/// Get the OAuth application secret from the JSON secret file.
fn secret_from_file() -> anyhow::Result<oauth2::ApplicationSecret> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("google-calendar-rsvp")
        .context("can't get XDG base directory")?;

    let f = fs::File::open(
        &xdg_dirs.find_data_file("oauth-secret.json")
            .context(format!(
                "Missing OAuth2 secret file. Create an application on the Google Developer Console (https://console.developers.google.com/) and download the JSON secret file to '{}'.",
                xdg_dirs.get_data_home()
                    .join("oauth-secret.json")
                    .display()
            ))?,
    )
        .context("unable to open OAuth secret file")?;

    let console_secret: oauth2::ConsoleApplicationSecret =
        serde_json::from_reader(f)
            .context("unable to parse OAuth application secret file")?;

    console_secret.installed
        .ok_or(anyhow::anyhow!("OAuth2 application secret not found"))
}

/// Extract an event ID from a base64-encoded eid.
fn event_id_from_base64(event_id: &str) -> anyhow::Result<String> {
    let decoded = match base64::decode(event_id) {
        Ok(d) => d,
        Err(_) => return Ok(event_id.to_owned()),
    };
    let id_email_pair = str::from_utf8(&decoded)
        .context("can't parse decoded base64 to UTF-8")?;
    let values = id_email_pair.split(" ").collect::<Vec<_>>();
    let id = values.first()
        .ok_or(
            anyhow::anyhow!("unable to extract event ID from '{}'", id_email_pair),
        )?
        .to_string();

    Ok(id)
}

/// Extract an eid from a Google Calendar invitation email.
fn eid_from_email(email: &[u8]) -> anyhow::Result<String> {
    let email = mailparse::parse_mail(&email)
        .context("unable to parse email")?;
    let re = Regex::new("eid=([^&]+)&")
        .context("can't compile email eid regex")?;

    // Assume email is multipart/alternative.
    for part in &email.subparts {
        if part.ctype.mimetype == "multipart/alternative" {
            for part in &part.subparts {
                if part.ctype.mimetype == "text/plain" {
                    let body = part.get_body()
                        .context("unable to get email body")?;
                    let captures = re.captures(&body)
                        .ok_or(anyhow::anyhow!("no matches for event ID"))?;
                    let eid = captures.get(1)
                        .ok_or(anyhow::anyhow!("event ID not found"))?;

                    return Ok(eid.as_str().to_owned());
                }
            }
        }
    }

    Err(anyhow::anyhow!("unable to extract event ID from email"))
}

/// Print a formatted event to standard output.
fn print_event(event: &Event) -> anyhow::Result<()> {
    if let Some(summary) = &event.summary {
        println!("{}", summary);
        println!();
    }

    if let Some(description) = &event.description {
        println!("{}", description);
    }

    if let Some(start) = &event.start {
        if let Some(date_time) = &start.date_time {
            let start_time = DateTime::parse_from_rfc3339(&date_time)
                .context("can't parse start time")?;
            print!("When         {}", start_time.format("%a %b %e, %Y %H:%M"));

            if let Some(end) = &event.end {
                if let Some(date_time) = &end.date_time {
                    let end_time = DateTime::parse_from_rfc3339(&date_time)
                        .context("can't parse end time")?;
                    print!(" – {}", end_time.format("%H:%M"));
                }
            }

            print!(" {}", start_time.format("%z"));
            println!();
        }
    }

    if let Some(conference_data) = &event.conference_data {
        if let Some(entry_points) = &conference_data.entry_points {
            for entry_point in entry_points {
                if let Some(uri) = &entry_point.uri {
                    println!("Joining info {}", uri);

                    break;
                }
            }
        }
    }

    if let Some(attendees) = &event.attendees {
        println!("Who");

        for attendee in attendees {
            let name = if let Some(display_name) = &attendee.display_name {
                display_name
            } else if let Some(email) = &attendee.email {
                email
            } else {
                continue
            };

            if let Some(response_status) = &attendee.response_status {
                match response_status.as_ref() {
                    "needsAction" =>
                        println!("             {}", name),
                    "declined" =>
                        println!("       No    {}", name),
                    "tentative" =>
                        println!("       Maybe {}", name),
                    "accepted" =>
                        println!("       Yes   {}", name),

                    _ => (),
                }
            }
        }
    }

    Ok(())
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
