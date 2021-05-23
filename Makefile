# Copyright (c) 2021  Teddy Wing
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.


VERSION := $(shell egrep '^version = ' Cargo.toml | awk -F '"' '{ print $$2 }')
TOOLCHAIN := $(shell fgrep default_host_triple $(HOME)/.rustup/settings.toml | awk -F '"' '{ print $$2 }')

SOURCES := $(shell find src -name '*.rs')
RELEASE_PRODUCT := target/release/google-calendar-rsvp

MAN_PAGE := doc/google-calendar-rsvp.1

DIST := $(abspath dist)
DIST_PRODUCT := $(DIST)/bin/google-calendar-rsvp
DIST_MAN_PAGE := $(DIST)/share/man/man1/google-calendar-rsvp.1


$(RELEASE_PRODUCT): $(SOURCES)
	cargo build --release


.PHONY: doc
doc: $(MAN_PAGE)

$(MAN_PAGE): $(MAN_PAGE).txt
	a2x --no-xmllint --format manpage $<


.PHONY: dist
dist: $(DIST_PRODUCT) $(DIST_MAN_PAGE)

$(DIST):
	mkdir -p $@

$(DIST)/bin: | $(DIST)
	mkdir -p $@

$(DIST)/share/man/man1: | $(DIST)
	mkdir -p $@

$(DIST_PRODUCT): $(RELEASE_PRODUCT) | $(DIST)/bin
	cp $< $@

$(DIST_MAN_PAGE): $(MAN_PAGE) | $(DIST)/share/man/man1
	cp $< $@


.PHONY: pkg
pkg: google-calendar-rsvp_$(VERSION)_$(TOOLCHAIN).tar.bz2

google-calendar-rsvp_$(VERSION)_$(TOOLCHAIN).tar.bz2: dist
	tar cjv -s /dist/google-calendar-rsvp_$(VERSION)_$(TOOLCHAIN)/ -f $@ dist
