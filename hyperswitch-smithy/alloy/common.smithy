$version: "2.0"

namespace alloy.common

/// ISO 3166-1 Alpha-2 country code
/// Full list in https://www.iso.org/obp/ui/#search/code/
/// example: "AF", "US"
@trait(
    selector: ":test(string, member > string)"
)
structure countryCodeFormat {}

/// Email address as defined in https://www.rfc-editor.org/rfc/rfc2821 and https://www.rfc-editor.org/rfc/rfc2822
/// A more human-readable format is available here: https://www.rfc-editor.org/rfc/rfc3696#section-3
@trait(
    selector: ":test(string, member > string)"
)
structure emailFormat {}

/// A hex triplet representing a RGB color code
/// example: "#09C" (short) or "#0099CC" (full)
@trait(
    selector: ":test(string, member > string)"
)
structure hexColorCodeFormat {}

/// ISO 639-1 Alpha-2 language code or Language for short.
/// Column `ISO 639-1` in https://www.loc.gov/standards/iso639-2/php/English_list.php
/// example: "fr", "en"
@trait(
    selector: ":test(string, member > string)"
)
structure languageCodeFormat {}

/// BCP 47 Language Tag
/// IETF RFC: https://tools.ietf.org/search/bcp47
/// example: "fr-CA", "en-US"
@trait(
    selector: ":test(string, member > string)"
)
structure languageTagFormat {}
