PRODUCT := flow-collector
DEBEMAIL := Hyperi Support <edgestream-support@hyperi.io>
CONTENTS := Netflow Collector - contact edgestream-support@hyperi.io for support enquiries
MAJOR_VERSION ?= 0.1.0

# Normalize SOURCE_DATE_EPOCH to *integer seconds* (handles float/ms/µs/ns)
EPOCHSEC := ${shell \
  if [ -n "$$SOURCE_DATE_EPOCH" ]; then \
    s="$$SOURCE_DATE_EPOCH"; \
    s=$${s%%.*}; \
    if [ "$$s" -ge 1000000000000000000 ] 2>/dev/null; then s=$$(( s/1000000000 )); \
    elif [ "$$s" -ge 1000000000000000 ] 2>/dev/null; then s=$$(( s/1000000 )); \
    elif [ "$$s" -ge 1000000000000 ] 2>/dev/null; then s=$$(( s/1000 )); \
    fi; \
    echo $$s; \
  else \
    date +%s; \
  fi}

# Latest reachable tag name (if any)
GIT_TAG := $(shell git describe --tags --abbrev=0 2>/dev/null)

# If annotated tag: use tagger date; else fall back to the tagged commit date
TAG_DATE := $(shell [ -n "$(GIT_TAG)" ] && git for-each-ref --format='%(taggerdate:format:%Y%m%d)' refs/tags/$(GIT_TAG) 2>/dev/null)
TAG_COMMIT := $(shell [ -n "$(GIT_TAG)" ] && git rev-list -n1 $(GIT_TAG) 2>/dev/null)
TAG_COMMIT_DATE := $(shell [ -n "$(TAG_COMMIT)" ] && git show -s --date=format:%Y%m%d --pretty=%cd $(TAG_COMMIT) 2>/dev/null)

# Head commit date
HEAD_DATE := $(shell git log -1 --date=format:%Y%m%d --pretty=%cd 2>/dev/null)

# Final date piece (YYYYMMDD) with normalized epoch fallback
DATE_YYYYMMDD := $(or \
  $(strip $(TAG_DATE)), \
  $(strip $(TAG_COMMIT_DATE)), \
  $(strip $(HEAD_DATE)), \
  ${shell date -u -d @$(EPOCHSEC) +%Y%m%d} \
)

# Version: MAJOR.gitYYYYMMDD.EPOCHSEC
MINOR_VERSION := git$(DATE_YYYYMMDD)
VERSION := $(MAJOR_VERSION).$(MINOR_VERSION).$(EPOCHSEC)

# RFC2822 date for changelog
DATE := ${shell date -u -d @$(EPOCHSEC) '+%a, %d %b %Y %H:%M:%S +0000'}

.EXPORT_ALL_VARIABLES:


clean_build:
	rm -rf ../*deb ./dist ./.pybuild ./build .flow-collector.egg-info
	rm -rf ./debian/.debhelper ./debian/debhelper-build-stamp
	rm -rf ./debian/flow-collector.postrm.debhelper ./debian/flow-collector.substvars
	rm -rf ./debian/files ./debian/flow-collector

clean: clean_build
.PHONY: clean

create_changelog:
	rm -f ./debian/changelog
	dch --create --package $(PRODUCT) --newversion $(VERSION) --urgency medium "$(CONTENTS)" --distribution unstable
	perl -0777 -pe 's/^ -- .+$$/ -- $(DEBEMAIL)  $(DATE)/m' -i ./debian/changelog

build_deb:
	chmod +x "debian/rules"
	SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) \
	debuild --no-lintian -i -uc -us -b -j4

build: build_deb
.PHONY: build

prepare: create_changelog
.PHONY: prepare

dist: clean prepare build

all: dist
.PHONY: all
