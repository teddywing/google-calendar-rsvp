google-calendar-rsvp
====================

RSVP to a Google Calendar event invitation from the command line.

In March 2021, [Google announced that RSVP hyperlinks in Google Calendar
invitation emails would require authentication][sign-in to RSVP announcement].
Previously, I had been using this script to accept calendar invitations in Mutt:

``` shell
#!/bin/sh

# google-calendar-invite-accept.sh
# Given a Google Calendar invite email on standard input, accept the invite.

EX_DATAERR=65

ampersand='%26'
equal_sign='%3D'

accept_invite='1'


accept_url=$($HOME/.mutt/scripts/extract_url/extract_url.pl <&0 |
	fgrep "${ampersand}rst${equal_sign}${accept_invite}${ampersand}" |
	perl -MURI::Escape -e 'print uri_unescape(<STDIN>)')

if [ -z "$accept_url" ]; then
	echo >&2 'google-calendar-invite-accept.sh: error: no acceptance URL'
	exit $EX_DATAERR
fi

curl -L "$accept_url" |
	w3m -dump -T text/html
```

Since the authentication change, I was forced to open the acceptance link in a
browser. This program enables you to RSVP to the invitation without leaving your
email client.

[sign-in to RSVP announcement]: https://workspaceupdates.googleblog.com/2021/03/sign-in-to-rsvp-via-hyperlinks-in.html


## Usage

	$ google-calendar-rsvp --yes 1g4j1h67ndq7kddrb2bptp2cua
	$ google-calendar-rsvp --maybe \
		1g4j1h67ndq7kddrb2bptp2cua \
		MWc0ajFoNjduZHE3a2RkcmIyYnB0cDJjdWEgcm9yeS5tZXJjdXJ5QGV4YW1wbGUuY28K
	$ google-calendar-rsvp --email --no < invitation.eml


## Authentication
In order to authenticate with Google and RSVP to calendar invitations:

1. Create a new project on the [Google Developer Console]
2. Add the Google Calendar API to the project
3. Create an OAuth 2.0 client ID
4. Download the OAuth JSON client secret file to
   `$XDG_DATA_HOME/google-calendar-rsvp/oauth-secret.json`


[Google Developer Console]: https://console.developers.google.com/


## Install
On Mac OS X, Google Calendar RSVP can be installed with Homebrew:

	$ brew install teddywing/formulae/google-calendar-rsvp

To compile from source or install on other platforms:

	$ cargo install --git https://github.com/teddywing/google-calendar-rsvp.git


## Uninstall

	$ cargo uninstall google-calendar-rsvp


## License
Copyright © 2021 Teddy Wing. Licensed under the GNU GPLv3+ (see the included
COPYING file).
