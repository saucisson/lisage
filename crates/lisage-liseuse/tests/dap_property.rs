//! D6-3 property tier (proptest, dev-dependency approved on #175):
//! codec invariants of the DAP JSON serializer and Content-Length framing
//! over arbitrary inputs.

use std::io::BufReader;

use lisage_liseuse::dap::{json_to_string, read_frame};
use lisage_liseuse::json::{self, Json};
use proptest::prelude::*;

proptest! {
    /// Whatever the string (quotes, backslashes, control chars, unicode),
    /// serializing and parsing it back yields the identical value.
    #[test]
    fn prop_json_string_roundtrip(text in ".*") {
        let value = Json::Str(text);
        let serialized = json_to_string(&value);
        let parsed = json::parse(&serialized).unwrap();
        prop_assert_eq!(parsed, value);
    }

    /// Object keys are escaped too: any key roundtrips through the codec.
    #[test]
    fn prop_json_key_roundtrip(key in ".*") {
        let value = Json::Obj(vec![(key, Json::Bool(true))]);
        let serialized = json_to_string(&value);
        let parsed = json::parse(&serialized).unwrap();
        prop_assert_eq!(parsed, value);
    }

    /// A Content-Length frame decodes back to the exact JSON body sent,
    /// for arbitrary (serializable) payloads.
    #[test]
    fn prop_frame_roundtrip(text in ".*", number in i64::MIN..i64::MAX) {
        let body = json_to_string(&Json::Obj(vec![
            ("text".to_string(), Json::Str(text)),
            ("number".to_string(), Json::Num(number)),
        ]));
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = BufReader::new(std::io::Cursor::new(frame.into_bytes()));
        let decoded = read_frame(&mut reader).unwrap().unwrap();
        prop_assert_eq!(decoded, json::parse(&body).unwrap());
    }
}
