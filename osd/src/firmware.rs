//! The firmware descriptor of OSD 0.4 (`schemas/0.4/firmware.schema.json`): a
//! firmware, mod or script a device can run, and what it changes.

use crate::json::{closed, list, list_of, obj, object, words, Bool, Opt, STRING};
use crate::sound::{Author, ForDevice};
use crate::{SchemaUrl, Version};

words! {
    /// What a firmware descriptor describes.
    FirmwareKind { Firmware = "firmware", Mod = "mod", Script = "script", Extension = "extension", }
}

words! {
    /// Where a firmware's users meet.
    CommunityKind {
        Forum = "forum", Discord = "discord", Chat = "chat", Subreddit = "subreddit", Wiki = "wiki", Site = "site",
    }
}

object! {
    /// A firmware, mod or script a device can run: who makes it, where it
    /// lives, its licence, what it changes to the device's MIDI, how it is
    /// installed.
    Firmware {
        "$schema" schema: Option<Version> = Opt(SchemaUrl("firmware")),
        "name" name: String = STRING,
        "version" version: String = STRING,
        /// Firmware when absent.
        "kind" kind: Option<FirmwareKind> = Opt(closed()),
        "device" device: ForDevice = obj(),
        /// Released by the device's maker; false when absent.
        "official" official: Option<bool> = Opt(Bool),
        "author" author: Option<Vec<Author>> = Opt(list_of(obj(), 1, usize::MAX)),
        "repository" repository: Option<String> = Opt(STRING),
        "homepage" homepage: Option<String> = Opt(STRING),
        "release" release: Option<Release> = Opt(obj()),
        /// An SPDX licence id, `public-domain`, or `none`.
        "license" license: Option<String> = Opt(STRING),
        "based_on" based_on: Option<BasedOn> = Opt(obj()),
        "midi" midi: Option<FirmwareMidi> = Opt(obj()),
        /// How the device shows which firmware it runs.
        "identify" identify: Option<String> = Opt(STRING),
        "install" install: Option<Install> = Opt(obj()),
        "description" description: Option<String> = Opt(STRING),
        "communities" communities: Option<Vec<Community>> = Opt(list(obj())),
    }
}

object! {
    /// Where a version is downloaded, from its author.
    Release {
        "url" url: Option<String> = Opt(STRING),
        "date" date: Option<String> = Opt(STRING),
    }
}

object! {
    /// The firmware it was forked or built from.
    BasedOn {
        "name" name: String = STRING,
        "version" version: Option<String> = Opt(STRING),
        "url" url: Option<String> = Opt(STRING),
    }
}

object! {
    /// What a firmware understands over MIDI.
    FirmwareMidi {
        /// The OSD device descriptor of its MIDI.
        "descriptor" descriptor: Option<String> = Opt(STRING),
        /// When it answers to another firmware's MIDI unchanged, that descriptor.
        "same_as" same_as: Option<String> = Opt(STRING),
        /// What it adds or changes, in words.
        "changes" changes: Option<String> = Opt(STRING),
    }
}

object! {
    /// How a firmware is installed, and how to go back to the maker's.
    Install {
        "method" method: Option<String> = Opt(STRING),
        "steps" steps: Option<String> = Opt(STRING),
        "revert" revert: Option<String> = Opt(STRING),
    }
}

object! {
    /// Where a firmware's users meet.
    Community {
        "name" name: String = STRING,
        "url" url: String = STRING,
        "kind" kind: CommunityKind = closed(),
    }
}
