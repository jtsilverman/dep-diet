use crate::analyzer::Suggestion;
use std::collections::HashMap;

struct Alt {
    alternative: &'static str,
    reason: &'static str,
}

fn alternatives_map() -> HashMap<&'static str, Alt> {
    let mut m = HashMap::new();
    m.insert("moment", Alt { alternative: "dayjs", reason: "4.2MB → 7KB gzip, mostly compatible API" });
    m.insert("moment-timezone", Alt { alternative: "dayjs + dayjs/plugin/timezone", reason: "Much smaller with timezone support" });
    m.insert("lodash", Alt { alternative: "lodash-es or native JS", reason: "Tree-shakeable, or most utils built into ES2022+" });
    m.insert("underscore", Alt { alternative: "native JS", reason: "Most utilities are built into modern JS" });
    m.insert("request", Alt { alternative: "undici or native fetch", reason: "Deprecated since 2020, use built-in fetch" });
    m.insert("axios", Alt { alternative: "ky or undici", reason: "Lighter HTTP clients, native fetch for simple cases" });
    m.insert("chalk", Alt { alternative: "picocolors", reason: "14KB → 2KB, 2x faster" });
    m.insert("colors", Alt { alternative: "picocolors", reason: "Was compromised in supply chain attack, use picocolors" });
    m.insert("uuid", Alt { alternative: "crypto.randomUUID()", reason: "Built into Node 19+ and all modern browsers" });
    m.insert("node-fetch", Alt { alternative: "native fetch", reason: "Built into Node 18+" });
    m.insert("commander", Alt { alternative: "citty or clipanion", reason: "Lighter CLI parsers" });
    m.insert("yargs", Alt { alternative: "citty or clipanion", reason: "Lighter CLI parsers" });
    m.insert("left-pad", Alt { alternative: "String.prototype.padStart()", reason: "Built into JS since ES2017" });
    m.insert("is-number", Alt { alternative: "typeof x === 'number'", reason: "One-line native check" });
    m.insert("is-odd", Alt { alternative: "x % 2 !== 0", reason: "One-line native check" });
    m
}

pub fn get_suggestions(package_names: &[String]) -> Vec<Suggestion> {
    let map = alternatives_map();
    let mut suggestions = Vec::new();

    for name in package_names {
        if let Some(alt) = map.get(name.as_str()) {
            suggestions.push(Suggestion {
                package: name.clone(),
                alternative: alt.alternative.to_string(),
                reason: alt.reason.to_string(),
            });
        }
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moment_suggests_dayjs() {
        let suggestions = get_suggestions(&["moment".to_string(), "express".to_string()]);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].package, "moment");
        assert_eq!(suggestions[0].alternative, "dayjs");
    }

    #[test]
    fn test_no_suggestions_for_unknown() {
        let suggestions = get_suggestions(&["express".to_string(), "fastify".to_string()]);
        assert!(suggestions.is_empty());
    }
}
